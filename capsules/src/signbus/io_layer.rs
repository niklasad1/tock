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
use signbus::{support, port_layer};

pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 1024] = [0; 1024];
pub static mut BUFFER2: [u8; 255] = [4; 255];

pub struct SignbusIOInterface<'a> {
    port_layer:				&'a port_layer::PortLayer,
	
    this_device_address:	Cell<u8>,
    sequence_number:		Cell<u16>,

	
	message_fragmented:		Cell<bool>,

    
	slave_write_buf:		TakeCell <'static, [u8]>,
    data_buf:				TakeCell <'static, [u8]>,
}

impl<'a> SignbusIOInterface<'a> {
    pub fn new(port_layer: 	&'a port_layer::PortLayer,
	       slave_write_buf:	&'static mut [u8],
	       data_buf:		&'static mut [u8]) -> SignbusIOInterface <'a> {

	SignbusIOInterface {
	    port_layer:		port_layer,
	    
		this_device_address:	Cell::new(0),
	    sequence_number:		Cell::new(0),
		message_fragmented:		Cell::new(false),
		
	    slave_write_buf:		TakeCell::new(slave_write_buf),
	    data_buf:				TakeCell::new(data_buf),
	}
    }


    // Host-to-network short (packages certain data into header)
    fn htons(&self, a: u16) -> u16 {
		return ((a as u16 & 0x00FF) << 8) | ((a as u16 & 0xFF00) >> 8);
    }

    fn get_message(&self,
		   recv_buf: &'static mut [u8],
		   encrypted: bool,
		   src_address: u8) {

	let mut done: u8 = 0;
    }

    /// Initialization routine to set up the slave address for this device.
    /// MUST be called before any other methods.
    pub fn signbus_io_init(&self, address: u8) -> ReturnCode {
		debug!("io_layer_init");

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
		debug!("Signbus_Interface_send");
		
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

	    	// Packet
	    	let mut packet: support::Packet = support::Packet {
	        	header: header,
	        	data:   &mut data[0..support::I2C_MAX_DATA_LEN],
	    	};
			
			let rc = self.port_layer.i2c_master_write(dest, packet, support::I2C_MAX_LEN);	
			if rc != ReturnCode::SUCCESS {return rc;}

		}
		else {

	    	// Packet
	    	let mut packet: support::Packet = support::Packet {
	        	header: header,
	        	data:   &mut data[0..len],
	    	};
			
			let rc = self.port_layer.i2c_master_write(dest, packet, len + support::HEADER_SIZE);	
			if rc != ReturnCode::SUCCESS {return rc;}
		}

		ReturnCode::SUCCESS
    }

	pub fn signbus_io_recv(&self, max_len: usize) -> ReturnCode {
		//debug!("io_layer_recv");
		let rc = self.port_layer.i2c_slave_listen(max_len);
		if rc != ReturnCode::SUCCESS {return rc;}

		// get_message() helper
		ReturnCode::SUCCESS
    }
}


impl<'a> signbus::port_layer::PortLayerClient for SignbusIOInterface <'a> {
	fn packet_received(&self, packet: signbus::support::Packet, error: signbus::support::Error) {
		debug!("PortLayerClient packet_received in io_layer");
    }

    fn packet_sent(&self, mut packet: support::Packet, error: isize) {
		debug!("PortLayerClient packet_sent in io_layer");
		//check error

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
			// callback protocol_layer

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

