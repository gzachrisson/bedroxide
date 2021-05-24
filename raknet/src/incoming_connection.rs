use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq)]
pub struct IncomingConnection {
    addr: SocketAddr,
    guid: u64,
}

impl IncomingConnection {
    pub(crate) fn new(addr: SocketAddr, guid: u64) -> Self {
        IncomingConnection { addr, guid }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn guid(&self) -> u64 {
        self.guid
    }
}