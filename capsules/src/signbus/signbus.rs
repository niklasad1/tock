/// Signbus implementation in the kernel level

use core::cell::Cell;
use core::cmp;
use kernel::{AppId, AppSlice, Callback, Driver, ReturnCode, Shared};
use kernel::common::take_cell::{MapCell, TakeCell};
use kernel::hil;
use kernel::hil::gpio;
use kernel::hil::i2c;
use kernel::ReturnCode

enum initialization_state {
	Start = 0,
	Isolated = 1,
	KeyExchange = 2,
	Done = 3,
} initialization_state_t;

enum initialization_message_type {
	InitializationDeclare = 0,
	InitializationKeyExchange = 1,
	InitializationGetMods = 2,
}

enum module_address {
	ModuleAddressController = 0x20,
	ModuleAddressStorage = 0x21,
	ModuleAddressRadio = 0x22,
} module_address_t;

pub struct Signbus {
	slave_address: Cell<u8>,
}

impl Signbus {
	const fn new() -> Signbus {
		Signbus {
			i2c: &'a I2CDevice,
			slave_address: Cell::new(0),
		}
	}

	fn signbus_io_init(&self, i2c_address: u8) -> ReturnCode {
		self.slave_address.set(i2c_address);
		port_signpost_init(i2c_address);

		return ReturnCode::SUCCESS;
	}

	fn port_signpost_init( i2c_address: u8) -> ReturnCode {
		

	}
	

}


impl hil::i2c for SignBus {


}


impl hil::gpio for SignBus {

}




