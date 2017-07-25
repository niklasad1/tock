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
use signbus;
use signbus::{support, io_layer, protocol_layer, app_layer};

//pub static mut BUFFER0: [u8; 1024] = [0; 1024];
//pub static mut BUFFER1: [u8; 1024] = [0; 1024];

pub trait ProtocolLayerClient {
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

pub struct SignbusProtocolLayer<'a> {
	io_layer: 	&'a io_layer::SignbusIOLayer<'a>,
	

	client: Cell<Option<&'static app_layer::SignbusAppLayer<'static>>>,
	
	//buf0:					TakeCell <'static, [u8]>,
	//buf1:					TakeCell <'static, [u8]>,
}

impl<'a> SignbusProtocolLayer<'a> {
	pub fn new(io_layer: 	&'a io_layer::SignbusIOLayer,
				//buf0:		&'static mut [u8],
				//buf1: 		&'static mut [u8]
	) -> SignbusProtocolLayer <'a> {
		
		SignbusProtocolLayer {
			io_layer:  			io_layer,
	       	
			client: 			Cell::new(None),
			
			//buf0:				TakeCell::new(buf0),
			//buf1:				TakeCell::new(buf1),
		}
	}

	pub fn set_client(&self, client: &'static app_layer::SignbusAppLayer) -> ReturnCode {
		self.client.set(Some(client));
		ReturnCode::SUCCESS
	}

	pub fn signbus_protocol_send(&self, dest: u8, data: &'static mut [u8], len: usize) -> ReturnCode {
		debug!("Signbus_Protocol send");
		
		// TODO: encryption not availabe in Rust
		let encrypted: bool = false;
	
		// Send to io_interface
		self.io_layer.signbus_io_send(dest, encrypted, data, len)
	}
	
	pub fn signbus_protocol_recv(&self, buffer: &'static mut [u8]) -> ReturnCode {
		debug!("Signbus_Protocol recv");

		self.io_layer.signbus_io_recv(buffer)
	}
}

impl<'a> io_layer::IOLayerClient for SignbusProtocolLayer <'a> {
    // Called when a new packet is received over I2C.
	fn packet_received(&self, data: &'static [u8], length: usize, error: support::Error) {

		// TODO: decryption not available in Rust
		self.client.get().map(|client| {
			client.packet_received(data, length, error);	
		});
		
	}

    // Called when an I2C master write command is complete.
    fn packet_sent(&self, error: support::Error) {

		self.client.get().map(|client| {
			client.packet_sent(error);	
		});

	}

    // Called when an I2C slave read has completed.
    fn packet_read_from_slave(&self) {}

    // Called when the mod_in GPIO goes low.
    fn mod_in_interrupt(&self) {}

    // Called when a delay_ms has completed.
    fn delay_complete(&self) {}
}
