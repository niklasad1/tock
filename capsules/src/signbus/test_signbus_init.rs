// Capsule to test signbus intialization functions in tock
// By: Justin Hsieh

use core::cell::Cell;
use core::cmp;
use kernel::{AppId, AppSlice, Callback, Driver, ReturnCode, Shared};
use kernel::common::take_cell::{MapCell, TakeCell};
use kernel::hil;
use kernel::hil::gpio;
use kernel::hil::time;
use signbus;
use signbus::{io_layer, support, app_layer, port_layer, protocol_layer};

pub static mut BUFFER0: [u8; 512] = [0; 512];



pub struct SignbusInitialization<'a> {

	// USE AS NEEDED
	app_layer: 			&'a app_layer::SignbusAppLayer<'a>,
	protocol_layer: 	&'a protocol_layer::SignbusProtocolLayer<'a>,
	io_layer: 			&'a io_layer::SignbusIOLayer<'a>,
	port_layer: 		&'a port_layer::PortLayer,

	app_client: Cell<Option<&'static app_layer::SignbusAppLayer<'static>>>,
	port_client: Cell<Option<&'static port_layer::PortLayer>>,

	buf:					TakeCell <'static, [u8]>,
}

impl<'a> SignbusInitialization <'a> {
	pub fn new(app_layer: 			&'a app_layer::SignbusAppLayer,
				protocol_layer: 	&'a protocol_layer::SignbusProtocolLayer,
				io_layer: 			&'a io_layer::SignbusIOLayer,
				port_layer: 		&'a port_layer::PortLayer,
				buf: 				&'static mut [u8],
	
	) -> SignbusInitialization <'a> {
		
		SignbusInitialization {
			app_layer:  		app_layer,
			protocol_layer:  	protocol_layer,
			io_layer:  			io_layer,
			port_layer:  		port_layer,
			
			app_client:			Cell::new(None),
			port_client:		Cell::new(None),
			buf:				TakeCell::new(buf),
		}
	}

	pub fn signpost_initialization_request_isolation(&self) {
		debug!("Request I2C isolation");
		// intialize mod out/in gpio
		self.port_layer.mod_out_set();
		self.port_layer.debug_led_off();
		self.port_layer.mod_in_enable_interrupt();

		// pull mod out low to signal controller
		// wait on controller interrupt on mod_in
		self.port_layer.mod_out_clear();
		self.port_layer.debug_led_on();
	}


	pub fn signpost_initialization_module_init(&self, i2c_address: u8) {
		debug!("Start Initialization");
		// intialize lower layers
		self.io_layer.signbus_io_init(i2c_address);
	
		// listen for messages
		self.buf.take().map(|buf| {	
			self.app_layer.signbus_app_recv(buf);
		});

		// communicate with controller and request 1:1 talk (isolation)
		self.signpost_initialization_request_isolation();
	}

}



impl<'a> port_layer::PortLayerClient2 for SignbusInitialization <'a> {
	
    // Called when the mod_in GPIO goes low.
    fn mod_in_interrupt(&self) {
		debug!("Interrupt!");

		self.port_layer.delay_ms(50);

		self.port_layer.mod_in_disable_interrupt();
		self.port_layer.mod_out_set();
		self.port_layer.debug_led_off();
	}

    // Called when a delay_ms has completed.
    fn delay_complete(&self) {
		debug!("Delay fired");
		self.mod_in_interrupt(); // need some kind of state to go to the correct functions after delay

	}

}


