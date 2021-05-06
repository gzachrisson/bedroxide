use log::debug;

use crate::internal_packet::{InternalPacket, SplitPacketHeader};

pub struct SplitPacketHandler {    
}

impl SplitPacketHandler {
    pub fn new() -> SplitPacketHandler {
        SplitPacketHandler {}
    }

    pub fn handle_split_packet(&mut self, header: SplitPacketHeader, packet: InternalPacket) -> Option<InternalPacket> {
        debug!("Split packet. count={}, id={}, idx={}", header.split_packet_count(), header.split_packet_id(), header.split_packet_index());

        // TODO: Insert packet into list of fragments/split packets
        // TODO: If all fragments have arrived, return full packet
        // TODO: Send progress to user

        let _payload = packet.into_payload();
        None
    }
}