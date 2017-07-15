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

use signbus;

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

	client: Cell<Option<&'static signbus::io_layer::SignbusIOInterface<'static>>>,
}

pub trait PortLayerClient {
     // Called when a new packet is received over I2C.
     fn packet_received(&self, packet: signbus::support::Packet, error: signbus::support::Error);

     // Called when an I2C master write command is complete.
     fn packet_sent(&self);

     // Called when an I2C slave read has completed.
     fn packet_read_from_slave(&self);

     // Called when the mod_in GPIO goes low.
     fn mod_in_interrupt(&self);

     // Called when a delay_ms has completed.
     fn delay_complete(&self);
}

pub trait PortLayer {
     fn init(&self, i2c_address: u8) -> ReturnCode;
     fn i2c_master_write(&self, i2c_address: u8, packet: signbus::support::Packet) -> ReturnCode;
     fn i2c_slave_listen(&self, max_len: usize) -> ReturnCode;
     fn i2c_slave_read_setup(&self, buf: &[u8], len: usize) -> ReturnCode;
     fn mod_out_set(&self) -> ReturnCode;
     fn mod_out_clear(&self) -> ReturnCode;
     fn mod_in_read(&self) -> ReturnCode;
     fn mod_in_enable_interrupt(&self) -> ReturnCode;
     fn mod_in_disable_interrupt(&self) -> ReturnCode;
     fn delay_ms(&self) -> ReturnCode;
     fn debug_led_on(&self) -> ReturnCode;
     fn debug_led_off(&self) -> ReturnCode;
}

impl<'a, A: hil::time::Alarm+'a> SignbusPortLayer<'a, A> {
     pub fn new(i2c: &'a hil::i2c::I2CMasterSlave,
		i2c_buffer: &'static mut [u8; 255],
		mod_in_pin: &'a hil::gpio::Pin,
		mod_out_pin: &'a hil::gpio::Pin,
		alarm: &'a A,
		debug_led: Option<&'a hil::gpio::Pin>,
		) -> SignbusPortLayer<'a, A> {

	  SignbusPortLayer {
	       i2c: i2c,
	       i2c_buffer: TakeCell::new(i2c_buffer),
	       mod_in_pin: mod_in_pin,
	       mod_out_pin: mod_out_pin,
	       alarm: alarm,
	       debug_led: Cell::new(debug_led),
	       client: Cell::new(None),
	  }
     }
}

impl<'a, A: hil::time::Alarm+'a> PortLayer for SignbusPortLayer<'a, A> {
     fn init(&self, i2c_address: u8) -> ReturnCode {
	  /*
	  let r = self.set_slave_address(i2c_address);
	  if r == ReturnCode::SUCCESS {
	       self.state.set(State::Init);
	  }

	  return r;
	  */
	  ReturnCode::SUCCESS
     }

     // Do a write to another I2C device
     fn i2c_master_write(&self, i2c_address: u8, packet: signbus::support::Packet) -> ReturnCode {
	  /*

	  debug!("Signbus_Port_master_write");

	  self.master_tx_buffer.take().map(|buffer|{
	       hil::i2c::I2CMaster::enable(self.i2c);
	       hil::i2c::I2CMaster::write(self.i2c, address, buffer, len as u8);
	  });

	  // TODO: yield() or implement client callback

	  self.state.set(State::MasterWrite);
	  return ReturnCode::SUCCESS;
	  */
	  ReturnCode::SUCCESS
     }


     // Listen for messages to this device as a slave.
     fn i2c_slave_listen(&self, max_len: usize) -> ReturnCode {
	  /*

	  self.slave_rx_buffer.take().map(|buffer| {
	       hil::i2c::I2CSlave::write_receive(self.i2c, buffer, 255);
	  });

	  hil::i2c::I2CSlave::enable(self.i2c);
	  hil::i2c::I2CSlave::listen(self.i2c);


	  self.state.set(State::SlaveRead);
	  return ReturnCode::SUCCESS;
	  */
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
	  ReturnCode::SUCCESS
     }

     fn mod_out_clear(&self) -> ReturnCode {
	  ReturnCode::SUCCESS
     }

     fn mod_in_read(&self) -> ReturnCode {
	  ReturnCode::SUCCESS
     }

     fn mod_in_enable_interrupt(&self) -> ReturnCode {
	  ReturnCode::SUCCESS
     }

     fn mod_in_disable_interrupt(&self) -> ReturnCode {
	  ReturnCode::SUCCESS
     }

     fn delay_ms(&self) -> ReturnCode {
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
	  debug!("I2CHwMasterClient command_complete for SignbusPortLayer");
     }
}

/// Handle I2C Slave callbacks.
impl<'a, A: hil::time::Alarm+'a> hil::i2c::I2CHwSlaveClient for SignbusPortLayer<'a, A> {
     fn command_complete(&self, buffer: &'static mut [u8], length: u8, transmission_type: hil::i2c::SlaveTransmissionType) {
	  debug!("I2CHwSlaveClient command_complete for SignbusPortLayer");
     }

     fn read_expected(&self) {
	  debug!("I2CHwSlaveClient read_expected for SignbusPortLayer");
     }

     fn write_expected(&self) {
	  debug!("I2CHwSlaveClient write_expected for SignbusPortLayer");
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

