use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq)]
pub struct SendReceipt {
    addr: SocketAddr,
    guid: u64,
    receipt: u32,
}

impl SendReceipt {
    pub(crate) fn new(addr: SocketAddr, guid: u64, receipt: u32) -> Self {
        SendReceipt { addr, guid, receipt }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn guid(&self) -> u64 {
        self.guid
    }

    pub fn receipt(&self) -> u32 {
        self.receipt
    }
}