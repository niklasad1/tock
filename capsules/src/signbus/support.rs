/// Helper code for Signbus

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
}

#[derive(Copy, Clone, Debug)]
pub struct Packet {
}

impl Packet {
    //XXX: need to figure out how to size this buffer right
    pub fn serialize_packet(packet: Packet, buf: &[u8]) -> &[u8] {
	buf
    }

    //XXX: need to figure out how to size this buffer right
    pub fn unserialize_packet(buf: &[u8]) -> Packet {
	Packet{
	}
    }
}

