use std::net::SocketAddr;

#[derive(Debug, PartialEq)]
pub struct Packet {
    addr: SocketAddr,
    guid: u64,
    payload: Box<[u8]>,
}

impl Packet {
    pub(crate) fn new(addr: SocketAddr, guid: u64, payload: Box<[u8]>) -> Self {
        Packet {
            addr,
            guid,
            payload,
        }        
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn guid(&self) -> u64 {
        self.guid
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}
