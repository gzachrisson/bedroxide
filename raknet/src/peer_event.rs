use crate::{IncomingConnection, Packet, SendReceipt};

#[derive(Debug, PartialEq)]
pub enum PeerEvent {
    Packet(Packet),
    SendReceiptAcked(SendReceipt),
    SendReceiptLoss(SendReceipt),
    IncomingConnection(IncomingConnection),
}