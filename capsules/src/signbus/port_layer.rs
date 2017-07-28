//! Kernel implementation of port_signpost_tock
//! apps/libsignpost/port_signpost_tock.c -> kernel/tock/capsules/src/port_signpost_tock.rs
//! By: Justin Hsieh
//!
//! Usage
//! -----
//!
//! ```rust
//! let port_layer = static_init!(
//! 	capsules::signbus::port_layer::SignbusPortLayer<'static, 
//!     	VirtualMuxAlarm<'static, sam4l::ast::Ast>>,
//!     capsules::signbus::port_layer::SignbusPortLayer::new(
//!        	&sam4l::i2c::I2C1,
//!        	&mut capsules::signbus::port_layer::I2C_BUFFER,
//!        	&sam4l::gpio::PB[14], // D0 mod_in
//!        	&sam4l::gpio::PB[15], // D1 mod_out
//!        	signbus_virtual_alarm,
//!        	Some(&sam4l::gpio::PA[13]), // RED LED
//!  )); 
//!    
//! ```


use core::cell::Cell;
use kernel::ReturnCode;
use kernel::common::take_cell::TakeCell;
use kernel::hil;
use kernel::hil::time::Frequency;
use kernel::hil::gpio;
use kernel::hil::gpio::{Pin, PinCtl, InputMode, InterruptMode};

//Capsules
use signbus;
use signbus::{io_layer, support, test_signbus_init};

/// Buffer to use for I2C messages. Messages are at most 255 bytes in length.
pub static mut I2C_BUFFER: [u8; 255] = [0; 255];

/// Signbus port layer. Implements hardware functionality for Signbus.
pub struct SignbusPortLayer<'a, A: hil::time::Alarm+'a> {
	i2c: &'a hil::i2c::I2CMasterSlave,
	i2c_buffer: TakeCell<'static, [u8]>,

	mod_in_pin: &'a hil::gpio::Pin,
	mod_out_pin: &'a hil::gpio::Pin,

	alarm: &'a A,

	debug_led: Cell<Option<&'a hil::gpio::Pin>>,
	
	init_client: Cell<Option<&'static test_signbus_init::SignbusInitialization<'static>>>,
	io_client: Cell<Option<&'static io_layer::SignbusIOLayer<'static>>>,

	listening:		Cell<bool>,
	master_action:	Cell<support::MasterAction>,
}

/// PortLayerClient for I2C sending/receiving callbacks. Implemented by SignbusIOLayer.
pub trait PortLayerClient {
     // Called when a new packet is received over I2C.
     fn packet_received(&self, packet: support::Packet, length: u8, error: support::Error);

     // Called when an I2C master write command is complete.
     fn packet_sent(&self, packet: support::Packet, error: support::Error);

     // Called when an I2C slave read has completed.
     fn packet_read_from_slave(&self);
}

/// PortLayerClient for GPIO and timer callbacks. Implemented by SignbusInitialization.
pub trait PortLayerClient2 {

     // Called when the mod_in GPIO goes low.
     fn mod_in_interrupt(&self);

     // Called when a delay_ms has completed.
     fn delay_complete(&self);

}

pub trait PortLayer {
	fn init(&self, i2c_address: u8) -> ReturnCode;
	fn i2c_master_write(&self, i2c_address: u8, packet: support::Packet, len: usize) -> ReturnCode;
	fn i2c_slave_listen(&self) -> ReturnCode;
	fn i2c_slave_read_setup(&self, buf: &[u8], len: usize) -> ReturnCode;
	fn mod_out_set(&self) -> ReturnCode;
	fn mod_out_clear(&self) -> ReturnCode;
	fn mod_in_read(&self) -> usize;
	fn mod_in_enable_interrupt(&self) -> ReturnCode;
	fn mod_in_disable_interrupt(&self) -> ReturnCode;
	fn delay_ms(&self, time: u32) -> ReturnCode;
	fn debug_led_on(&self) -> ReturnCode;
	fn debug_led_off(&self) -> ReturnCode;
}
	
impl<'a, A: hil::time::Alarm+'a> SignbusPortLayer<'a, A> {
	pub fn new(i2c: &'a hil::i2c::I2CMasterSlave,
		i2c_buffer: &'static mut [u8; 255],
		mod_in_pin: &'a hil::gpio::Pin,
		mod_out_pin: &'a hil::gpio::Pin,
		alarm: &'a A,
		debug_led: Option<&'a hil::gpio::Pin>, ) -> SignbusPortLayer<'a, A> {

	  	SignbusPortLayer {
	       	i2c: i2c,
	       	i2c_buffer: TakeCell::new(i2c_buffer),
	      	mod_in_pin: mod_in_pin,
	       	mod_out_pin: mod_out_pin,
	       	alarm: alarm,
	       	debug_led: Cell::new(debug_led),
			init_client: Cell::new(None),
	       	io_client: Cell::new(None),
			listening: Cell::new(false),
			master_action:	Cell::new(support::MasterAction::Write),
		}
    }
	
	pub fn set_io_client(&self, client: &'static io_layer::SignbusIOLayer) -> ReturnCode {
		self.io_client.set(Some(client));
		ReturnCode::SUCCESS
	}
	
	pub fn set_init_client(&self, client: &'static test_signbus_init::SignbusInitialization) -> ReturnCode {
		self.init_client.set(Some(client));
		ReturnCode::SUCCESS
	}
}

impl<'a, A: hil::time::Alarm+'a> PortLayer for SignbusPortLayer<'a, A> {
	// Set address for this device
	fn init(&self, i2c_address: u8) -> ReturnCode {
		
		if i2c_address > 0x7f {
			return ReturnCode::EINVAL;
		}
		hil::i2c::I2CSlave::set_address(self.i2c, i2c_address);
		ReturnCode::SUCCESS
	}

    // Do a write to another I2C device
	fn i2c_master_write(&self, i2c_address: u8, packet: support::Packet, len: usize) -> ReturnCode {
		
		self.i2c_buffer.take().map(|buffer| {
			// packet -> buffer
			support::serialize_packet(packet, len-support::HEADER_SIZE, buffer);
	    	
			hil::i2c::I2CMaster::enable(self.i2c);
	    	hil::i2c::I2CMaster::write(self.i2c, i2c_address, buffer, len as u8);
	  	});

		self.master_action.set(support::MasterAction::Write);
	  	ReturnCode::SUCCESS
    }

    // Listen for messages to this device as a slave.
	fn i2c_slave_listen(&self) -> ReturnCode {
		
		self.i2c_buffer.take().map(|buffer| {
	    	hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255);
	  	});

	  	hil::i2c::I2CSlave::enable(self.i2c);
	  	hil::i2c::I2CSlave::listen(self.i2c);

		self.listening.set(true);

	  	ReturnCode::SUCCESS
	}

	fn i2c_slave_read_setup(&self, buf: &[u8], len: usize) -> ReturnCode {
		
		// TODO: implement slave write/ master read
		// self.master_action.set(support::MasterAction::Read(len as u8));
		// hil::i2c::I2CSlave::read_send(self.i2c, buffer, len as u8);
		ReturnCode::SUCCESS
	}

	fn mod_out_set(&self) -> ReturnCode {
		self.mod_out_pin.make_output();
		self.mod_out_pin.set();
		ReturnCode::SUCCESS
	}

	fn mod_out_clear(&self) -> ReturnCode {
		self.mod_out_pin.make_output();
		self.mod_out_pin.clear();
 		ReturnCode::SUCCESS
	}

	fn mod_in_read(&self) -> usize {
		let pin_state = self.mod_in_pin.read();
		//ReturnCode::SuccessWithValue {value: pin_state as usize}
		return pin_state as usize;
	}

	fn mod_in_enable_interrupt(&self) -> ReturnCode {
		self.mod_in_pin.make_input();	
		self.mod_in_pin.enable_interrupt(0, gpio::InterruptMode::FallingEdge);
		ReturnCode::SUCCESS
	}

	fn mod_in_disable_interrupt(&self) -> ReturnCode {	
        self.mod_in_pin.disable_interrupt();
        self.mod_in_pin.disable();
		ReturnCode::SUCCESS
	}

	fn delay_ms(&self, time: u32) -> ReturnCode {
		let interval = time * <A::Frequency>::frequency() / 1000;
		let tics = self.alarm.now().wrapping_add(interval);
		self.alarm.set_alarm(tics);
		ReturnCode::SUCCESS	
	}

	fn debug_led_on(&self) -> ReturnCode {
		self.debug_led.get().map(|led| {
			led.make_output();
			led.set();
		});
 		ReturnCode::SUCCESS
	}

	fn debug_led_off(&self) -> ReturnCode {
		self.debug_led.get().map(|led| {
			led.make_output();
			led.clear();
		});
 		ReturnCode::SUCCESS
	}
}

/// Handle I2C Master callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::i2c::I2CHwMasterClient for SignbusPortLayer<'a, A> {
	// Master read or write completed. 	
	fn command_complete(&self, buffer: &'static mut [u8], error: hil::i2c::Error) {
	
        let err: support::Error = match error {										// userland error code
            hil::i2c::Error::AddressNak => support::Error::AddressNak, 				// -1
            hil::i2c::Error::DataNak => support::Error::DataNak,					// -2
            hil::i2c::Error::ArbitrationLost => support::Error::ArbitrationLost, 	// -3
            hil::i2c::Error::CommandComplete => support::Error::CommandComplete, 	//  0
        };

        match self.master_action.get() {
            support::MasterAction::Write => {
               	
				self.io_client.get().map(move |io_client| {
					let packet = support::unserialize_packet(buffer);
					self.i2c_buffer.replace(buffer);
					io_client.packet_sent(packet, err);	
				});
				
			}

            support::MasterAction::Read(read_len) => {
				// TODO: implement slave write/ master read
			}
		}

		// Check to see if we were listening as an I2C slave and should re-enable that mode
        if self.listening.get() {
            hil::i2c::I2CSlave::enable(self.i2c);
            hil::i2c::I2CSlave::listen(self.i2c);
        }
	}
}

/// Handle I2C Slave callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::i2c::I2CHwSlaveClient for SignbusPortLayer<'a, A> {
	// Slave write_receive or read_send completed
	fn command_complete(&self, buffer: &'static mut [u8], length: u8, transmission_type: hil::i2c::SlaveTransmissionType) {
		
		match transmission_type {
            hil::i2c::SlaveTransmissionType::Read => {
            	//TODO: implement slave write/ master read
			}

			// Master write/ slave read
            hil::i2c::SlaveTransmissionType::Write => {
				self.io_client.get().map(move |io_client| {
					let packet = support::unserialize_packet(buffer);
					self.i2c_buffer.replace(buffer);
					io_client.packet_received(packet, length, support::Error::CommandComplete);	
				});
            }
        }
     }

    fn read_expected(&self) {
		// TODO: implement slave write/ master read
		debug!("I2CHwSlaveClient read_expected for SignbusPortLayer");
    }

	// Slave received message, but does not have buffer. Call write_receive again to initiate callback.
	fn write_expected(&self) {
		self.i2c_buffer.take().map(|buffer| { 
			hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255); 
		});
     }
}

/// Handle alarm callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::time::Client for SignbusPortLayer<'a, A> {
	// Timer done
	fn fired(&self) {
		self.init_client.get().map(|init_client|{
			init_client.delay_complete();
		});
    }
}

/// Handle GPIO callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::gpio::Client for SignbusPortLayer<'a, A> {
	// Interrupt
	fn fired(&self, _: usize) {
		self.init_client.get().map(|init_client|{
			init_client.mod_in_interrupt();
		});
	}
}

