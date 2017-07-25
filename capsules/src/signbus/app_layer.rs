#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/// Kernel implementation of signbus_app_layer
/// apps/libsignpost/signbus_app_layer.c -> kernel/tock/capsules/src/signbus_app_layer.rs
/// By: Justin Hsieh


use core::cell::Cell;
use core::cmp;
use kernel::{AppId, AppSlice, Callback, Driver, ReturnCode, Shared};
use kernel::common::take_cell::{MapCell, TakeCell};
use kernel::hil;
use kernel::hil::gpio;
use kernel::hil::time;
// Capsules
use signbus::protocol_layer;

pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 256] = [0; 256];


pub struct App {
	callback: Option<Callback>,
	master_tx_buffer: Option<AppSlice<Shared, u8>>,
	master_rx_buffer: Option<AppSlice<Shared, u8>>,
	slave_tx_buffer: Option<AppSlice<Shared, u8>>,
	slave_rx_buffer: Option<AppSlice<Shared, u8>>,
}

impl Default for App {
	fn default() -> App {
		App {
			callback: None,
			master_tx_buffer: None,
			master_rx_buffer: None,
			slave_tx_buffer: None,
			slave_rx_buffer: None,
		}
	}
}

pub enum SignbusFrameType {
    NotificationFrame = 0,
    CommandFrame = 1,
    ResponseFrame = 2,
    ErrorFrame = 3,
}

pub enum SignbusApiType {
    InitializationApiType = 1,
    StorageApiType = 2,
    NetworkingApiType = 3,
    ProcessingApiType = 4,
    EnergyApiType = 5,
    TimeLocationApiType = 6,
    EdisonApiType = 7,
    JsonApiType = 8,
    WatchdogApiType = 9,
    HighestApiType = 10,
}

pub struct SignbusAppLayer<'a> {
	protocol_layer: 	&'a protocol_layer::SignbusProtocolLayer<'a>,
	payload:					TakeCell <'static, [u8]>,
    app: 						MapCell<App>,
}

impl<'a> SignbusAppLayer<'a,> {
	pub fn new(protocol_layer: &'a protocol_layer::SignbusProtocolLayer,
				payload: &'static mut [u8]) -> SignbusAppLayer <'a> {
		
		SignbusAppLayer {
			protocol_layer:  	protocol_layer,
			payload:					TakeCell::new(payload),
            app: 						MapCell::new(App::default()),
		}
	}

	pub fn signbus_app_send(&self, 
							address: u8,
							frame_type: SignbusFrameType,
							api_type: SignbusApiType,
							message_type: u8,
							message_length: usize,
							message: &'static mut [u8]) -> ReturnCode {
		
		debug!("Signbus_App_send");
		
		let mut rc = ReturnCode::SUCCESS;
		let len: usize = 1 + 1 + 1 + message_length;
		
		// Concatenate info onto message
		// TODO: Greather than 256 could panic
		self.payload.map(|payload|{
			payload[0] = frame_type as u8;
			payload[1] = api_type as u8;
			payload[2] = message_type;
			
			let d = &mut payload.as_mut()[3..len as usize];
			for (i, c) in message[0..message_length as usize].iter().enumerate() {
				d[i] = *c;
			}	
		});

		// Send to protocol_layer
		self.payload.take().map(|payload|{
			rc = self.protocol_layer.signbus_protocol_send(address, payload, len);
		});

		return rc;
	}
	
}
