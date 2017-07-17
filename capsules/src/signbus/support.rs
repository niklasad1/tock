/// Helper code for Signbus

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
}


/// Signbus Packet 
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

//#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Packet {
	header: SignbusNetworkHeader,
	data: &'static mut [u8],
}

impl Packet {
    //XXX: need to figure out how to size this buffer right
    pub fn serialize_packet(packet: Packet, buf: &mut [u8]) -> &[u8] {

		// Network Flags	
		buf[0] = packet.header.flags.is_fragment as u8; 
		buf[1] = packet.header.flags.is_encrypted as u8; 
		buf[2] = packet.header.flags.rsv_wire_bit5 as u8; 
		buf[3] = packet.header.flags.rsv_wire_bit4 as u8; 
		buf[4] = packet.header.flags.version;

		buf[5] = packet.header.src;	
		buf[6] = (packet.header.sequence_number & 0x0F) as u8;
		buf[7] = (packet.header.sequence_number & 0xF0 >> 8) as u8;
		buf[8] = (packet.header.length & 0x0F) as u8;
		buf[9] = (packet.header.length & 0xF0 >> 8) as u8;
		buf[10] = (packet.header.fragment_offset & 0x0F) as u8;
		buf[11] = (packet.header.fragment_offset & 0xF0 >> 8) as u8;

		buf 
    }

    //XXX: need to figure out how to size this buffer right
    //pub fn unserialize_packet(buf: &[u8]) -> Packet {
	//Packet{
	//}
    //}
}

