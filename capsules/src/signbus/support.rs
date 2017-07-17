/// Helper code for Signbus

pub const I2C_MAX_LEN: usize = 255;
pub const HEADER_SIZE: usize = 12;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
}


/// Signbus Packet 
#[repr(C, packed)]
pub struct SignbusNetworkFlags {
    pub is_fragment:	bool,
    pub is_encrypted:	bool,
    pub rsv_wire_bit5:	bool,
    pub rsv_wire_bit4:	bool,
    pub version:		u8,
}

#[repr(C, packed)]
pub struct SignbusNetworkHeader {
    pub flags:				SignbusNetworkFlags,
    pub src:				u8,
    pub sequence_number:	u16,
    pub length:				u16,
    pub fragment_offset:	u16,
}

//#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Packet {
	pub header: SignbusNetworkHeader,
	pub data: &'static mut [u8],
}

//impl Packet {
    //XXX: need to figure out how to size this buffer right
    pub fn serialize_packet(packet: Packet, data_len: usize, buf: &mut [u8]) -> &[u8] {

		// Network Flags	
		buf[0] = packet.header.flags.is_fragment as u8; 
		buf[1] = packet.header.flags.is_encrypted as u8; 
		buf[2] = packet.header.flags.rsv_wire_bit5 as u8; 
		buf[3] = packet.header.flags.rsv_wire_bit4 as u8; 
		buf[4] = packet.header.flags.version;

		buf[5] = packet.header.src;	
		buf[6] = (packet.header.sequence_number & 0x00FF) as u8;
		buf[7] = ((packet.header.sequence_number & 0xFF00) >> 8) as u8;
		buf[8] = (packet.header.length & 0x00FF) as u8;
		buf[9] = ((packet.header.length & 0xFF00) >> 8) as u8;
		buf[10] = (packet.header.fragment_offset & 0x00FF) as u8;
		buf[11] = ((packet.header.fragment_offset & 0xFF00) >> 8) as u8;

			
		//let d = &mut buf.as_mut()[12..data_len+12];
		for (i, c) in packet.data[0..data_len].iter().enumerate() {
		    buf[i+12] = *c;
		}
		
		//debug!("{:?}", buf);

		buf 
    }

    //XXX: need to figure out how to size this buffer right
    //pub fn unserialize_packet(buf: &[u8]) -> Packet {
	//Packet{
	//}
    //}
//}

