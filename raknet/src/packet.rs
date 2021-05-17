use std::net::SocketAddr;

use crate::OrderingChannelIndex;

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

#[derive(Debug, PartialEq)]
pub enum Reliability {
    Unreliable,
    Reliable,
}

#[derive(Debug, PartialEq)]
pub enum Ordering {
    None,
    Ordered(OrderingChannelIndex),
    Sequenced(OrderingChannelIndex),
}

#[derive(Debug, PartialEq)]
pub enum Priority {
    /// The highest possible priority.
    Highest,
    /// For every 2 Immediate priority packet 1 High priority packet will be sent.
    High,
    /// For every 2 High priority packet 1 Medium priority packet will be sent.
    Medium,
    /// For every 2 Medium priority packet 1 Low priority packet will be sent.
    Low,
}