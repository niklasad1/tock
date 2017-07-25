#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/// Kernel implementation of signbus_io_interface
/// apps/libsignpost/signbus_io_interface.c -> kernel/tock/capsules/src/signbus_io_interface.rs
/// By: Justin Hsieh

use core::mem;
use core::slice;
use core::cell::Cell;
use kernel::{ReturnCode};
use kernel::common::take_cell::{MapCell, TakeCell};
use kernel::hil;

use signbus;
use signbus::{support, port_layer, protocol_layer};

pub static mut BUFFER0: [u8; 1024] = [0; 1024];
pub static mut BUFFER1: [u8; 1024] = [0; 1024];
pub static mut BUFFER2: [u8; 512] = [4; 512];

pub struct SignbusIOLayer<'a> {
    port_layer:				&'a port_layer::PortLayer,
	
    this_device_address:	Cell<u8>,
    sequence_number:		Cell<u16>,
			

	message_seq_no:			Cell<u16>,
	message_src:     		Cell<u8>,		
	length_received:		Cell<usize>,

	client: Cell<Option<&'static protocol_layer::SignbusProtocolLayer<'static>>>,
	
	recv_buf:				TakeCell <'static, [u8]>,
    data_buf:				TakeCell <'static, [u8]>,
}

pub trait IOLayerClient {
     // Called when a new packet is received over I2C.
     fn packet_received(&self, data: &'static [u8], length: usize, error: support::Error);

     // Called when an I2C master write command is complete.
     fn packet_sent(&self, error: support::Error);

     // Called when an I2C slave read has completed.
     fn packet_read_from_slave(&self);

     // Called when the mod_in GPIO goes low.
     fn mod_in_interrupt(&self);

     // Called when a delay_ms has completed.
     fn delay_complete(&self);
}

impl<'a> SignbusIOLayer<'a> {
    pub fn new(port_layer: 	&'a port_layer::PortLayer,
	       recv_buf:	&'static mut [u8],
	       data_buf:		&'static mut [u8]) -> SignbusIOLayer <'a> {

		SignbusIOLayer {
		    port_layer:		port_layer,
		    
			this_device_address:		Cell::new(0),
		    sequence_number:			Cell::new(0),
			
			message_seq_no:				Cell::new(0),
			message_src:     			Cell::new(0),
			length_received:			Cell::new(0),

			client: 					Cell::new(None),
			
		    recv_buf:					TakeCell::new(recv_buf),
		    data_buf:					TakeCell::new(data_buf),
		}
    }

	pub fn set_client(&self, client: &'static protocol_layer::SignbusProtocolLayer) -> ReturnCode {
		self.client.set(Some(client));
		ReturnCode::SUCCESS
	}
   
	/// Initialization routine to set up the slave address for this device.
    /// MUST be called before any other methods.
    pub fn signbus_io_init(&self, address: u8) -> ReturnCode {
		//debug!("io_layer_init");

		self.this_device_address.set(address);
		self.port_layer.init(address);

		return ReturnCode::SUCCESS;
    }


	/// Synchronous send call
    pub fn signbus_io_send(&self, dest: u8, encrypted: bool, data: &'static mut [u8], len: usize) -> ReturnCode {
		//debug!("Signbus_Interface_send");
		
		self.sequence_number.set(self.sequence_number.get() + 1);

		// Network Flags
	    let flags: support::SignbusNetworkFlags = support::SignbusNetworkFlags {
	        is_fragment:    (len + support::HEADER_SIZE) > support::I2C_MAX_LEN,
	        is_encrypted:   encrypted,
	        rsv_wire_bit5:  false,
	        rsv_wire_bit4:  false,
	        version:        0x1,
	    };
	
	    // Network Header
	    let header: support::SignbusNetworkHeader = support::SignbusNetworkHeader {
	        flags:              flags,
	        src:                dest,
	        sequence_number:    self.sequence_number.get(),
	        length:             (support::HEADER_SIZE + len) as u16,
	        fragment_offset:    0,
	    };
	
		if header.flags.is_fragment {
			// Save all data in order to send in multiple packets	
			self.data_buf.map(|data_buf| {
				let d = &mut data_buf.as_mut()[0..len];
				for (i, c) in data[0..len].iter().enumerate() {
            		d[i] = *c;
            	}
			});

			// Copy data from slice into sized array to package into packet
			let mut data_copy: [u8; support::I2C_MAX_DATA_LEN] = [0; support::I2C_MAX_DATA_LEN];   
            for (i, c) in data[0..support::I2C_MAX_DATA_LEN].iter().enumerate() {
            	data_copy[i] = *c;
            }

	    	// Packet
	    	let packet: support::Packet = support::Packet {
	        	header: header,
	        	data:   data_copy,
	    	};
			
			let rc = self.port_layer.i2c_master_write(dest, packet, support::I2C_MAX_LEN);	
			if rc != ReturnCode::SUCCESS {return rc;}

		}
		else {
			// Copy data from slice into sized array to package into packet
			let mut data_copy: [u8; support::I2C_MAX_DATA_LEN] = [0; support::I2C_MAX_DATA_LEN];
            for (i, c) in data[0..len].iter().enumerate() {
            	data_copy[i] = *c;
            }

	    	// Packet
	    	let packet: support::Packet = support::Packet {
	        	header: header,
	        	data:   data_copy,
	    	};
			
			let rc = self.port_layer.i2c_master_write(dest, packet, len + support::HEADER_SIZE);	
			if rc != ReturnCode::SUCCESS {return rc;}
		}

		ReturnCode::SUCCESS
    }

	fn signbus_io_recv(&self, buffer: &'static mut [u8]) -> ReturnCode {
		debug!("io_layer_recv");
	
		self.recv_buf.replace(buffer);
	
		let rc = self.port_layer.i2c_slave_listen();
		if rc != ReturnCode::SUCCESS {return rc;}

		ReturnCode::SUCCESS
    }
}


impl<'a> signbus::port_layer::PortLayerClient for SignbusIOLayer <'a> {
	fn packet_received(&self, packet: support::Packet, length: u8, error: support::Error) {
		debug!("PortLayerClient packet_received in io_layer");

		// Error checking
		if error != support::Error::CommandComplete {
			// Callback protocol_layer
			self.client.get().map(|client| {
				self.recv_buf.take().map(|recv_buf| {
					client.packet_received(recv_buf, self.length_received.get(), error);	
				});
			});
			// Reset
			self.length_received.set(0);
			return
			// future: implement sending error message to source
		}
		
		// Record needed packet data
		let seq_no 	= packet.header.sequence_number;
		let src 	= packet.header.src;
		let more_packets = packet.header.flags.is_fragment;
		let offset 	= packet.header.fragment_offset as usize;
		let remainder = packet.header.length as usize-support::HEADER_SIZE-packet.header.fragment_offset as usize;

		// First packet
		if self.length_received.get() == 0 {
			// Save src and seq_no
			self.message_seq_no.set(seq_no);	
			self.message_src.set(src);
		}
		// Subsequent packets
		else {
			// If new src, drop current packet	
			if self.message_seq_no.get() != seq_no || self.message_src.get() != src {
				// Save new src and seq_no
				self.message_seq_no.set(seq_no);				
				self.message_src.set(src);				
				
				// Reset
				self.length_received.set(0);
			}
   		}

	
		// More packets	
		if more_packets == true {
			// Copy data and update length_received
			self.recv_buf.map(|recv_buf| {
				let d = &mut recv_buf.as_mut()[offset..offset+support::I2C_MAX_DATA_LEN];
				for (i, c) in packet.data[0..support::I2C_MAX_DATA_LEN].iter().enumerate() {
        			d[i] = *c;
				}
			});
			self.length_received.set(self.length_received.get() + support::I2C_MAX_DATA_LEN);
		} 	
		// Last packet
		else {
			// Copy data and update length_received
			self.recv_buf.map(|recv_buf| {
				let d = &mut recv_buf.as_mut()[offset..offset+remainder];
				for (i, c) in packet.data[0..remainder].iter().enumerate() {
        			d[i] = *c;
				}
			});
			self.length_received.set(self.length_received.get() + remainder);
			
			// sanity check
			if self.length_received.get() + support::HEADER_SIZE == packet.header.length as usize {
				debug!("this should happen");
			}
			
			// Callback protocol_layer 
			self.client.get().map(|client| {
				self.recv_buf.take().map(|recv_buf| {
					client.packet_received(recv_buf, self.length_received.get(), error);	
				});
			});

			// Reset	
			self.length_received.set(0);
		}

	}
		


    fn packet_sent(&self, mut packet: support::Packet, error: signbus::support::Error) {
		//debug!("PortLayerClient packet_sent in io_layer");
		
		// If error, stop sending and propogate up
		if error != support::Error::CommandComplete	{
			// Callback protocol_layer
			self.client.get().map(|client| {
				client.packet_sent(error);	
			});
		}

		if packet.header.flags.is_fragment {
			// Update fragment offset
			let offset = support::I2C_MAX_DATA_LEN + packet.header.fragment_offset as usize; 
			packet.header.fragment_offset = offset as u16;
		
			// Determines if this is last packet and update is_fragment
			let data_left_to_send = packet.header.length as usize - support::HEADER_SIZE - offset;
			let more_packets = data_left_to_send as usize > support::I2C_MAX_DATA_LEN;
			packet.header.flags.is_fragment = more_packets;
	
			if more_packets {
				// Copy next frame of data from data_buf into packet	
				self.data_buf.map(|data_buf| {
					let d = &mut data_buf.as_mut()[offset..offset+support::I2C_MAX_DATA_LEN];
					for (i, c) in packet.data[0..support::I2C_MAX_DATA_LEN].iter_mut().enumerate() {
            			*c = d[i];
            		}
				});
			
				self.port_layer.i2c_master_write(packet.header.src, packet, support::I2C_MAX_LEN);	

			} else {
				// Copy next frame of data from data_buf into packet	
				self.data_buf.map(|data_buf| {
					let d = &mut data_buf.as_mut()[offset..offset+data_left_to_send];
					for (i, c) in packet.data[0..data_left_to_send].iter_mut().enumerate() {
            			*c = d[i];
            		}
				});
				
				self.port_layer.i2c_master_write(packet.header.src, packet, data_left_to_send+support::HEADER_SIZE);	
			}

		} else {
			// Callback protocol_layer
			self.client.get().map(|client| {
				client.packet_sent(error);	
			});
		}		
	
    }

    fn packet_read_from_slave(&self) {
		debug!("PortLayerClient packet_read_from_slave in io_layer");
    }

    fn mod_in_interrupt(&self) {
		debug!("PortLayerClient mod_in_interrupt in io_layer");
    }

    fn delay_complete(&self) {
		debug!("PortLayerClient delay_complete in io_layer");
    }
}

