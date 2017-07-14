#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/// Kernel implementation of signbus_protocol_layer
/// apps/libsignpost/signbus_protocol_layer.c -> kernel/tock/capsules/src/signbus_protocol_layer.rs
/// By: Justin Hsieh

use core::cell::Cell;
use kernel::{AppId, AppSlice, Callback, Driver, ReturnCode, Shared};
use kernel::common::take_cell::{MapCell, TakeCell};
use kernel::hil;
// Capsules
use signbus::port_signpost_tock;
use signbus::signbus_io_interface;

pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 256] = [0; 256];
pub static mut BUFFER2: [u8; 256] = [1; 256];

pub struct SignbusProtocolLayer<'a> {
	signbus_io_interface: 	&'a signbus_io_interface::SignbusIOInterface<'a>,
	buf0:					TakeCell <'static, [u8]>,
	buf1:					TakeCell <'static, [u8]>,
}

impl<'a> SignbusProtocolLayer<'a,> {
	pub fn new(signbus_io_interface: &'a signbus_io_interface::SignbusIOInterface<'a>,
				buf0:		&'static mut [u8],
				buf1: 		&'static mut [u8]) -> SignbusProtocolLayer <'a> {
		
		SignbusProtocolLayer {
			signbus_io_interface:  		signbus_io_interface,
			buf0:				TakeCell::new(buf0),
			buf1:				TakeCell::new(buf1),
		}
	}

	pub fn signbus_protocol_send(&self, 
								address: u8,
								data: &'static mut [u8],
								len: u16) -> ReturnCode {
		
		debug!("Signbus_Protocol_send");
		// TODO: encryption not availabe in Rust
		let encrypted: bool = false;
	
		// Send to io_interface
		self.signbus_io_interface.signbus_io_send(address, encrypted, data, len)
	}
	
}
