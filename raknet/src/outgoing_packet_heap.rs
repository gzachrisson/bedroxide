use crate::{internal_packet::InternalPacket, packet::Priority};

pub struct OutgoingPacketHeap {
}

impl OutgoingPacketHeap {
    pub fn new() -> Self {
        OutgoingPacketHeap {            
        }
    }

    pub fn push(&mut self, _priority: Priority, _packet: InternalPacket) {
        // TODO: Store packet in a min-heap
    }
}