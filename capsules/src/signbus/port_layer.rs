#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
/// Kernel implementation of port_signpost_tock
/// apps/libsignpost/port_signpost_tock.c -> kernel/tock/capsules/src/port_signpost_tock.rs
/// By: Justin Hsieh

use core::cell::Cell;
use kernel::ReturnCode;
use kernel::common::take_cell::TakeCell;
use kernel::hil;
use kernel::hil::time::Frequency;
use kernel::hil::gpio;
use kernel::hil::gpio::{Pin, PinCtl, InputMode, InterruptMode};
use signbus;
use signbus::{io_layer, support};

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

	client: Cell<Option<&'static io_layer::SignbusIOLayer<'static>>>,

	listening:		Cell<bool>,
	master_action:	Cell<support::MasterAction>,
}

pub trait PortLayerClient {
     // Called when a new packet is received over I2C.
     fn packet_received(&self, packet: support::Packet, length: u8, error: support::Error);

     // Called when an I2C master write command is complete.
     fn packet_sent(&self, packet: support::Packet, error: support::Error);

     // Called when an I2C slave read has completed.
     fn packet_read_from_slave(&self);

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
	fn mod_in_read(&self) -> ReturnCode;
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
	       	client: Cell::new(None),
			listening: Cell::new(false),
			master_action:	Cell::new(support::MasterAction::Write),
		}
    }
	
	pub fn set_client(&self, client: &'static io_layer::SignbusIOLayer) -> ReturnCode {
		self.client.set(Some(client));
		ReturnCode::SUCCESS
	}

	// testing purposes
	pub fn delay(&self, time: u32) {
		self.delay_ms(time);
	}

	pub fn set(&self) {
		self.mod_out_set();
	}	
	
	pub fn clear(&self) {
		self.mod_out_clear();
	}
	
	pub fn enable_interrupt(&self) {
		self.mod_in_enable_interrupt();
	}	
}

impl<'a, A: hil::time::Alarm+'a> PortLayer for SignbusPortLayer<'a, A> {
	fn init(&self, i2c_address: u8) -> ReturnCode {
		//debug!("port_layer_init");
		
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
	    	
			//debug!("port_layer_master_write");
			hil::i2c::I2CMaster::enable(self.i2c);
	    	hil::i2c::I2CMaster::write(self.i2c, i2c_address, buffer, len as u8);
	  	});

		self.master_action.set(support::MasterAction::Write);
	  	ReturnCode::SUCCESS
     }


     // Listen for messages to this device as a slave.
	fn i2c_slave_listen(&self) -> ReturnCode {
		debug!("port_layer_slave_listen");
		
		self.i2c_buffer.take().map(|buffer| {
	    	hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255);
	  	});

	  	hil::i2c::I2CSlave::enable(self.i2c);
	  	hil::i2c::I2CSlave::listen(self.i2c);

		self.listening.set(true);

	  	ReturnCode::SUCCESS
	}

	fn i2c_slave_read_setup(&self, buf: &[u8], len: usize) -> ReturnCode {
	/*
		self.slave_tx_buffer.take().map(|buffer| {
			hil::i2c::I2CSlave::read_send(self.i2c, buffer, len as u8);
	  	});

	  	self.state.set(State::MasterRead);
	  	return ReturnCode::SUCCESS;
	*/
		ReturnCode::SUCCESS
	}

	fn mod_out_set(&self) -> ReturnCode {
 		debug!("port_layer_mod_out_set");
		self.mod_out_pin.make_output();
		self.mod_out_pin.set();
		ReturnCode::SUCCESS
	}

	fn mod_out_clear(&self) -> ReturnCode {
 		debug!("port_layer_mod_out_clear");
		self.mod_out_pin.make_output();
		self.mod_out_pin.clear();
 		ReturnCode::SUCCESS
	}

	fn mod_in_read(&self) -> ReturnCode {
 		debug!("port_layer_mod_in_read");
		let pin_state = self.mod_in_pin.read();
		ReturnCode::SuccessWithValue {value: pin_state as usize}
	}

	fn mod_in_enable_interrupt(&self) -> ReturnCode {
		debug!("port_layer_mod_in_enable_interupt");
 			
		self.mod_in_pin.make_input();	
		self.mod_in_pin.enable_interrupt(0, gpio::InterruptMode::FallingEdge);

		ReturnCode::SUCCESS
	}

	fn mod_in_disable_interrupt(&self) -> ReturnCode {	
 		debug!("port_layer_mod_in_disable_interupt");

        self.mod_in_pin.disable_interrupt();
        self.mod_in_pin.disable();
		ReturnCode::SUCCESS
	}

	fn delay_ms(&self, time: u32) -> ReturnCode {
		debug!("port_layer_delay: {}", time);
		
		let interval = time * <A::Frequency>::frequency() / 1000;
		let tics = self.alarm.now().wrapping_add(interval);
		self.alarm.set_alarm(tics);
		
		ReturnCode::SUCCESS	
	}

	fn debug_led_on(&self) -> ReturnCode {
 		ReturnCode::SUCCESS
	}

	fn debug_led_off(&self) -> ReturnCode {
 		ReturnCode::SUCCESS
	}
}

/// Handle I2C Master callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::i2c::I2CHwMasterClient for SignbusPortLayer<'a, A> {
	fn command_complete(&self, buffer: &'static mut [u8], error: hil::i2c::Error) {
		//debug!("I2CHwMasterClient command_complete for SignbusPortLayer");
	
        let err: support::Error = match error {										// userland error code
            hil::i2c::Error::AddressNak => support::Error::AddressNak, 				// -1
            hil::i2c::Error::DataNak => support::Error::DataNak,					// -2
            hil::i2c::Error::ArbitrationLost => support::Error::ArbitrationLost, 	// -3
            hil::i2c::Error::CommandComplete => support::Error::CommandComplete, 	//  0
        };

        match self.master_action.get() {
            support::MasterAction::Write => {
               	
				self.client.get().map(move |client| {
					let packet = support::unserialize_packet(buffer);
					self.i2c_buffer.replace(buffer);
					client.packet_sent(packet, err);	
				});
				
			}

            support::MasterAction::Read(read_len) => {
				// TODO: in the future
			}
		}

		// Check to see if we were listening as an I2C slave and should re-enable
        // that mode.
        if self.listening.get() {
            hil::i2c::I2CSlave::enable(self.i2c);
            hil::i2c::I2CSlave::listen(self.i2c);
        }
	}
}

/// Handle I2C Slave callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::i2c::I2CHwSlaveClient for SignbusPortLayer<'a, A> {
    // Slave received message, write_receive completed
	fn command_complete(&self, buffer: &'static mut [u8], length: u8, transmission_type: hil::i2c::SlaveTransmissionType) {
		debug!("I2CHwSlaveClient command_complete for SignbusPortLayer");
        match transmission_type {
            hil::i2c::SlaveTransmissionType::Write => {
            	//TODO: in the future
			}

            hil::i2c::SlaveTransmissionType::Read => {
				self.client.get().map(move |client| {
					let packet = support::unserialize_packet(buffer);
					self.i2c_buffer.replace(buffer);
					client.packet_received(packet, length, support::Error::CommandComplete);	
				});
            }
        }
     }

    fn read_expected(&self) {
		//debug!("I2CHwSlaveClient read_expected for SignbusPortLayer");
    }

	// Slave received message, but does not have buffer.
	fn write_expected(&self) {
		debug!("I2CHwSlaveClient write_expected for SignbusPortLayer");
        
		self.i2c_buffer.take().map(|buffer| { 
			hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255); 
		});
     }
}

/// Handle alarm callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::time::Client for SignbusPortLayer<'a, A> {
	fn fired(&self) {
	 	debug!("time::Client fired for SignbusPortLayer");
    }
}

/// Handle GPIO callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::gpio::Client for SignbusPortLayer<'a, A> {
	fn fired(&self, _: usize) {
	 	debug!("gpio::Client fired for SignbusPortLayer");
	}
}

