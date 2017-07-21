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

pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 1024] = [0; 1024];
pub static mut BUFFER2: [u8; 512] = [4; 512];

pub struct SignbusIOLayer<'a> {
    port_layer:				&'a port_layer::PortLayer,
	
    this_device_address:	Cell<u8>,
    sequence_number:		Cell<u16>,
			

	message_sequence_number:	Cell<u8>,
	message_source_address:     Cell<u16>,		
	length_received:			Cell<usize>,

	client: Cell<Option<&'static protocol_layer::SignbusProtocolLayer<'static>>>,
	
	slave_write_buf:		TakeCell <'static, [u8]>,
    data_buf:				TakeCell <'static, [u8]>,
}


pub trait IOLayerClient {
     // Called when a new packet is received over I2C.
     fn packet_received(&self, packet: support::Packet, error: support::Error);

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
	       slave_write_buf:	&'static mut [u8],
	       data_buf:		&'static mut [u8]) -> SignbusIOLayer <'a> {

		SignbusIOLayer {
		    port_layer:		port_layer,
		    
			this_device_address:		Cell::new(0),
		    sequence_number:			Cell::new(0),
			
			message_sequence_number:	Cell::new(0),
			message_source_address:     Cell::new(0),
			length_received:			Cell::new(0),

			client: 					Cell::new(None),
			
		    slave_write_buf:			TakeCell::new(slave_write_buf),
		    data_buf:					TakeCell::new(data_buf),
		}
    }

	pub fn set_client(&self, client: &'static protocol_layer::SignbusProtocolLayer) -> ReturnCode {
		self.client.set(Some(client));
		ReturnCode::SUCCESS
	}
   
	// testing purposes 
	pub fn init(&self, address: u8) -> ReturnCode {
		self.signbus_io_init(address);
		ReturnCode::SUCCESS

	}
    pub fn send(&self, dest: u8, encrypted: bool, data: &'static mut [u8], len: usize) -> ReturnCode {
		self.signbus_io_send(dest, encrypted, data, len);
		ReturnCode::SUCCESS
	}

	pub fn recv(&self) -> ReturnCode {
		self.signbus_io_recv();
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


    // synchronous send call
    pub fn signbus_io_send(&self,
			   			dest: u8,
			   			encrypted: bool,
			   			data: &'static mut [u8],
			   			len: usize) -> ReturnCode {
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

	fn signbus_io_recv(&self) -> ReturnCode {
		debug!("io_layer_recv");
		
		let rc = self.port_layer.i2c_slave_listen();
		if rc != ReturnCode::SUCCESS {return rc;}

		// get_message() helper
		ReturnCode::SUCCESS
    }
}


impl<'a> signbus::port_layer::PortLayerClient for SignbusIOLayer <'a> {
	fn packet_received(&self, packet: signbus::support::Packet, length: u8, error: signbus::support::Error) {
		debug!("PortLayerClient packet_received in io_layer");

		if error != support::Error::CommandComplete {
			// callback protocol_layer
			self.client.get().map(|client| {
				client.packet_received(packet, error);	
			});
		}

		// Maybe implement sending error message to source
		


    }

    fn packet_sent(&self, mut packet: support::Packet, error: signbus::support::Error) {
		//debug!("PortLayerClient packet_sent in io_layer");
		
		// If error, stop sending and propogate up
		if error != support::Error::CommandComplete	{
			// callback protocol_layer
			self.client.get().map(|client| {
				client.packet_sent(error);	
			});
		}

		if packet.header.flags.is_fragment {
			// Update sequence number
			let seq_no = self.sequence_number.get() + 1;
			packet.header.sequence_number = seq_no;
			self.sequence_number.set(seq_no);
	
			// Update fragment offset
			let offset = support::I2C_MAX_DATA_LEN + packet.header.fragment_offset as usize; 
			packet.header.fragment_offset = offset as u16;
		
			// Determines if this is last packet and update is_fragment
			let data_left_to_send = packet.header.length as usize - support::HEADER_SIZE - offset;
			let more_packets = data_left_to_send as usize > support::I2C_MAX_DATA_LEN;
			packet.header.flags.is_fragment = more_packets;
	
			if more_packets {
				//debug!("io_layer more packets");
				// Copy next frame of data from data_buf into packet	
				self.data_buf.map(|data_buf| {
					let d = &mut data_buf.as_mut()[offset..offset+support::I2C_MAX_DATA_LEN];
					for (i, c) in packet.data[0..support::I2C_MAX_DATA_LEN].iter_mut().enumerate() {
            			*c = d[i];
            		}
				});
			
				self.port_layer.i2c_master_write(packet.header.src, packet, support::I2C_MAX_LEN);	

			} else {
				//debug!("io_layer one more packet");
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
			// callback protocol_layer
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

