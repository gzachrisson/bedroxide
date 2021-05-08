use crate::Packet;

#[derive(Debug, PartialEq)]
pub enum PeerEvent {
    Packet(Packet),
}