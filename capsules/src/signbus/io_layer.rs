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
pub static mut BUFFER1: [u8; 256] = [0; 256];
pub static mut BUFFER2: [u8; 15] = [4; 15];

pub struct SignbusIOInterface<'a> {
    port_layer:				&'a port_layer::PortLayer,
    this_device_address:	Cell<u8>,
    sequence_number:		Cell<u16>,
    slave_write_buf:		TakeCell <'static, [u8]>,
    packet_buf:				TakeCell <'static, [u8]>,
}

impl<'a> SignbusIOInterface<'a> {
    pub fn new(port_layer: 	&'a port_layer::PortLayer,
	       slave_write_buf:	&'static mut [u8],
	       packet_buf:		&'static mut [u8]) -> SignbusIOInterface <'a> {

	SignbusIOInterface {
	    port_layer:		port_layer,
	    this_device_address:	Cell::new(0),
	    sequence_number:		Cell::new(0),
	    slave_write_buf:		TakeCell::new(slave_write_buf),
	    packet_buf:				TakeCell::new(packet_buf),
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
	        is_fragment:    false,
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
	        length:             support::HEADER_SIZE + len,
	        fragment_offset:    0,
	    };
	
	    // Packet
	    let mut packet: support::Packet = support::Packet {
	        header: header,
	        data:   data,
	    };

		let rc = self.port_layer.i2c_master_write(dest, packet, len);	
		if rc != ReturnCode::SUCCESS {return rc;}

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

    fn packet_sent(&self) {
		debug!("PortLayerClient packet_sent in io_layer");
		
		


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

