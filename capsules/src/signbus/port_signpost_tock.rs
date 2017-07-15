#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/// Kernel implementation of port_signpost_tock 
/// apps/libsignpost/port_signpost_tock.c -> kernel/tock/capsules/src/port_signpost_tock.rs
/// By: Justin Hsieh

use core::cell::Cell;
use kernel::{ReturnCode};
use kernel::common::take_cell::{TakeCell};
use kernel::hil;
use kernel::hil::i2c;

use signbus::signbus_io_interface;

// Buffers to use for I2C messages
pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 256] = [0; 256];
pub static mut BUFFER2: [u8; 256] = [0; 256];
pub static mut BUFFER3: [u8; 256] = [0; 256];

#[derive(Clone,Copy,PartialEq)]
enum MasterAction {
	Read(u8),
	Write,
}

/// States of the I2C protocol for Signbus
#[derive(Clone,Copy,PartialEq)]
enum State {
	Idle,
	Init,
	MasterWrite,
	MasterRead,
	SlaveWrite,
	SlaveRead,
}

pub trait PortSignpostTockClient {
	fn command_complete(&self, buffer: &'static mut [u8], error: hil::i2c::Error);
}

pub struct PortSignpostTock<'a> {
	i2c: 		&'a hil::i2c::I2CMasterSlave,
	
	pub master_tx_buffer:		TakeCell <'static, [u8]>,
	pub master_rx_buffer:		TakeCell <'static, [u8]>,
	pub slave_tx_buffer:		TakeCell <'static, [u8]>,
	pub slave_rx_buffer:		TakeCell <'static, [u8]>,

    client: 		Cell<Option<&'static signbus_io_interface::SignbusIOInterface<'static>>>,

	state:			Cell<State>,
    listening: 		Cell<bool>,
}

impl<'a> PortSignpostTock<'a> {
	pub fn new(	i2c: &'a hil::i2c::I2CMasterSlave,
				master_tx_buffer: &'static mut [u8],
				master_rx_buffer: &'static mut [u8],
				slave_tx_buffer: &'static mut [u8],
				slave_rx_buffer: &'static mut [u8]) -> PortSignpostTock<'a> {
		PortSignpostTock {
			i2c:  		i2c,
			master_tx_buffer:		TakeCell::new(master_tx_buffer),
			master_rx_buffer:		TakeCell::new(master_rx_buffer),
			slave_tx_buffer:		TakeCell::new(slave_tx_buffer),
			slave_rx_buffer:		TakeCell::new(slave_rx_buffer),
            client: 				Cell::new(None),
			state:					Cell::new(State::Idle),
     		listening: 				Cell::new(true),
		}
	}


    pub fn set_client(&self, client: &'static signbus_io_interface::SignbusIOInterface<'a>) {
        self.client.set(Some(client));
    }
	
	fn set_slave_address(&self, i2c_address: u8) -> ReturnCode {

		if i2c_address > 0x7f {
			return ReturnCode::EINVAL;
		}
		debug!("Set my slave address to: {}", i2c_address);
		hil::i2c::I2CSlave::set_address(self.i2c, i2c_address);

		return ReturnCode::SUCCESS;
	}
	
	pub fn init(&self, i2c_address: u8) -> ReturnCode {
		
		let r = self.set_slave_address(i2c_address);
		if r == ReturnCode::SUCCESS {
			self.state.set(State::Init);
		}

		return r;
	}

    // Do a write to another I2C device
	pub fn i2c_master_write(&self, address: u8, len: u16) -> ReturnCode {

		debug!("Signbus_Port_master_write");
		
		self.master_tx_buffer.take().map(|buffer|{
			hil::i2c::I2CMaster::enable(self.i2c);
			hil::i2c::I2CMaster::write(self.i2c, address, buffer, len as u8);
		});
		
		// TODO: yield() or implement client callback

		self.state.set(State::MasterWrite);
		return ReturnCode::SUCCESS;
	}

    
	// Listen for messages to this device as a slave.
	pub fn i2c_slave_listen(&self) -> ReturnCode {

    	debug!("Signbus_Port_slave_listen");
		self.slave_rx_buffer.take().map(|buffer| {
			hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255);
		});

		hil::i2c::I2CSlave::enable(self.i2c);
		hil::i2c::I2CSlave::listen(self.i2c);


		self.state.set(State::SlaveRead);
		return ReturnCode::SUCCESS;
	}

	pub fn i2c_slave_read_setup(&self, len: u32) -> ReturnCode {
		self.slave_tx_buffer.take().map(|buffer| {
			hil::i2c::I2CSlave::read_send(self.i2c, buffer, len as u8);
		});

		self.state.set(State::MasterRead);
		return ReturnCode::SUCCESS;	
	}

}


impl<'a> i2c::I2CHwMasterClient for PortSignpostTock <'a> {
	fn command_complete(&self, buffer: &'static mut [u8], error: hil::i2c::Error) {
    	debug!("I2CHwMasterClient for PortSignpostTock");

		//TODO: implement callback
/*
        // Map I2C error to a number we can pass back to the application
        let err: isize = match error {
            hil::i2c::Error::AddressNak => -1,
            hil::i2c::Error::DataNak => -2,
            hil::i2c::Error::ArbitrationLost => -3,
            hil::i2c::Error::CommandComplete => 0,
        };

        // Signal the application layer. Need to copy read in bytes if this
        // was a read call.
        match self.master_action.get() {
            MasterAction::Write => {
                self.master_buffer.replace(buffer);

                self.app.map(|app| {
                    app.callback.map(|mut cb| { cb.schedule(0, err as usize, 0); });
                });
            }

            MasterAction::Read(read_len) => {
                self.app.map(|app| {
                    app.master_rx_buffer.as_mut().map(move |app_buffer| {
                        let len = cmp::min(app_buffer.len(), read_len as usize);

                        let d = &mut app_buffer.as_mut()[0..(len as usize)];
                        for (i, c) in buffer[0..len].iter().enumerate() {
                            d[i] = *c;
                        }

                        self.master_buffer.replace(buffer);
                    });

                    app.callback.map(|mut cb| { cb.schedule(1, err as usize, 0); });
                });
            }
        }

        // Check to see if we were listening as an I2C slave and should re-enable
        // that mode.
        if self.listening.get() {
            hil::i2c::I2CSlave::enable(self.i2c);
            hil::i2c::I2CSlave::listen(self.i2c);
        }
    }

*/
	}
}


impl<'a> i2c::I2CHwSlaveClient for PortSignpostTock <'a> {
	fn command_complete(&self, 
						buffer: &'static mut [u8], 
						length: u8,
						transmission_type: hil::i2c::SlaveTransmissionType) {
		//TODO: implement callback
    	debug!("I2CHwSlaveClient for PortSignpostTock");
	}

	fn read_expected(&self) {
		//TODO:	
	}
	
	fn write_expected(&self) {
		//TODO:
	}

}



/*
impl<'a, A: time::Alarm + 'a> i2c::I2CClient for PortSignpostTock<'a, A> {
	// Link from I2C capsule to PortSignpostTock capsule
	// fn command_complete ()
}

impl<'a, A: time::Alarm + 'a> time::Client for PortSignpostTock<'a, A> [
	// Link from time capsule to PortSignpostTock capsule
	// fn fired ()
}

*/


