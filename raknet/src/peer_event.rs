use crate::{IncomingConnection, Packet, SendReceipt};

#[derive(Debug, PartialEq)]
pub enum PeerEvent {
    Packet(Packet),
    SendReceipt(SendReceipt),
    IncomingConnection(IncomingConnection),
}