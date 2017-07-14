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
// Capsules
use signbus::port_signpost_tock;
use signbus::port_signpost_tock::PortSignpostTockClient;

pub static mut BUFFER0: [u8; 256] = [0; 256];
pub static mut BUFFER1: [u8; 256] = [0; 256];
pub static mut BUFFER2: [u8; 256] = [1; 256];

static debug: u8 = 1;
const I2C_MAX_LEN: u16 = 255; 

////////////////////
// NETWORK PACKET //
////////////////////
#[repr(C, packed)]
pub struct SignbusNetworkFlags {
	is_fragment:	bool,
	is_encrypted:	bool,
	rsv_wire_bit5:	bool,
	rsv_wire_bit4:	bool,
	version:		u8,
}

#[repr(C, packed)]
pub struct SignbusNetworkHeader {
	flags:				SignbusNetworkFlags,
	src:				u8,
	sequence_number:	u16,
	length:				u16,
	fragment_offset:	u16,
}


#[repr(C, packed)]
pub struct Packet {
	header: SignbusNetworkHeader,
	data:	&'static mut [u8],	
}
////////////////////
// ************** //
////////////////////



////////////////////
//   NEW PACKET   //
////////////////////
pub struct NewPacket {
	new: bool,
	len: u16,
}
////////////////////
// ************** //
////////////////////


// Function needed to turn struct into [u8]
// Should be okay if structs are packed
unsafe fn as_byte_slice<'a, T>(input: &'a T) -> &'a [u8] {
    slice::from_raw_parts(input as *const T as *const u8, mem::size_of::<T>())
}

pub struct SignbusIOInterface<'a> {
	port_signpost_tock: 	&'a port_signpost_tock::PortSignpostTock<'a>,
	this_device_address:	Cell<u8>,
	sequence_number:		Cell<u16>,
	newpkt:					Cell<NewPacket>,
	slave_write_buf:		TakeCell <'static, [u8]>,
	packet_buf:				TakeCell <'static, [u8]>,
}

impl<'a> SignbusIOInterface<'a> {
	pub fn new(port_signpost_tock: &'a port_signpost_tock::PortSignpostTock<'a>,
				slave_write_buf:	&'static mut [u8],
				packet_buf: 		&'static mut [u8]) -> SignbusIOInterface <'a> {
	
		SignbusIOInterface {
			port_signpost_tock:  	port_signpost_tock,
			this_device_address:	Cell::new(0),
			sequence_number:		Cell::new(0),
			newpkt:					Cell::new(NewPacket {new: false,len: 0,}),
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
		
		self.this_device_address.set(address);
		self.port_signpost_tock.init(address);
		
		if debug == 1 {	
			//debug!("Address: {}", self.this_device_address.get());
			//debug!("SignbusNetworkHeader: {}", mem::size_of::<SignbusNetworkHeader>());
			//debug!("SignbusNetworkFlags: {}", mem::size_of::<SignbusNetworkFlags>());
		}

		return ReturnCode::SUCCESS;
	}


	// synchronous send call
	pub fn signbus_io_send(&self, 
							dest: u8, 
							encrypted: bool, 
							data: &'static mut [u8],
							len: u16) -> ReturnCode {

		debug!("Signbus_Interface_send");
		
		// update sequence number
		self.sequence_number.set(self.sequence_number.get() + 1);
		
		// size of header (sent everytime)
		let header_size: u16 = mem::size_of::<SignbusNetworkHeader>() as u16;
		// max data in one packet
		let max_data_len: u16 = I2C_MAX_LEN - header_size;
		// number of bytes left to be sent
		let mut to_send: u16 = len;
		// number of packets to be sent
		let mut num_packets: u16 = len/max_data_len + 1;		
		if len % max_data_len == 0 {
			num_packets = num_packets - 1; 
		}
	
		// Network Flags
		let flags: SignbusNetworkFlags = SignbusNetworkFlags {
			is_fragment:	false, // to_send > max_data_len
			is_encrypted:	encrypted,
			rsv_wire_bit5:	false,
			rsv_wire_bit4:	false,
			version:		0x1,
		};

		// Network Header
		let header: SignbusNetworkHeader = SignbusNetworkHeader {
			flags:				flags,
			src:				self.this_device_address.get(),
			sequence_number:	self.htons(self.sequence_number.get()),
			length:				self.htons(num_packets * header_size + len),
			fragment_offset:	self.htons(0),
		};

		// Packet
		let mut packet: Packet = Packet {
			header: header,
			data:	data,	
		};
		
		if debug == 1 {
			//debug!("{:?}", data.len());
			//debug!("{:?}", packet.data.len());
			//debug!("data length: {} ", packet.data.len());
			//debug!("to_send: {} ", to_send);
			//debug!("max_data_len: {} ", max_data_len);
			//debug!("header_size: {} ", header_size);
			//debug!("num_packets: {} ", num_packets);
		}
		
		
		//while to_send > 0 {
			let more_packets: bool = to_send > max_data_len;;
			let mut START: usize = (header_size + 1) as usize; 

			// UPDATE HEADER
			packet.header.flags.is_fragment = more_packets;
			packet.header.fragment_offset = self.htons(len-to_send);

			// COPY HEADER
			self.port_signpost_tock.master_tx_buffer.map(|port_buffer|{
				let d = &mut port_buffer.as_mut()[0..header_size as usize];
				let bytes: &[u8]= unsafe { as_byte_slice(&packet.header) };
				for (i, c) in bytes[0..header_size as usize].iter().enumerate() {
					d[i] = *c;
				}
			});

			// COPY DATA
			if more_packets == true {
				self.port_signpost_tock.master_tx_buffer.map(|port_buffer|{
					let d = &mut port_buffer.as_mut()[START..I2C_MAX_LEN as usize];
					for (i, c) in packet.data[0..(max_data_len-1) as usize].iter().enumerate() {
						d[i] = *c;
					}	
				});
			}
			else {
				self.port_signpost_tock.master_tx_buffer.map(|port_buffer|{
					let d = &mut port_buffer.as_mut()[START..(START as u16 + to_send) as usize];
					for (i, c) in packet.data[0..to_send as usize].iter().enumerate() {
						d[i] = *c;
					}	
				});
			}
		
			// SEND I2C message and update bytes left to send
			if more_packets == true {
				let rc = self.port_signpost_tock.i2c_master_write(dest, I2C_MAX_LEN);	
				if rc != ReturnCode::SUCCESS { return rc; } 
				to_send -= max_data_len;
			}
			else {
				let rc = self.port_signpost_tock.i2c_master_write(dest, to_send);	
				if rc != ReturnCode::SUCCESS { return rc; } 
				to_send = 0;
			}
		//}

		if debug == 1 {
			//debug!("End of Function");
		}

		ReturnCode::SUCCESS
	}

	pub fn signbus_io_recv(&self) -> ReturnCode {
		let rc = self.port_signpost_tock.i2c_slave_listen();
		if rc != ReturnCode::SUCCESS {return rc;}	
		
		// get_message() helper
		ReturnCode::SUCCESS
	}  
}


impl<'a> PortSignpostTockClient for SignbusIOInterface <'a> {
	fn command_complete(&self, buffer: &'static mut [u8], error: hil::i2c::Error) {
		debug!("PortSignpostTockClient in SignbusIOInterface");

	}	
}
