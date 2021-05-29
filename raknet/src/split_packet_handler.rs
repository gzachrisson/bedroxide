use std::{collections::HashMap, time::Instant};
use log::{debug, error};

use crate::{error::ReadError, internal_packet::{InternalOrdering, InternalPacket, InternalReliability}, Result};

struct SplitPacketChannel {
    /// The `InternalReliability` of the split packet when reassembled.
    reliability: InternalReliability,
    /// The `InternalOrdering` of the split packet when reassembled.
    ordering: InternalOrdering,
    /// The number of bytes received so far.
    received_byte_count: u32,
    /// The number of parts received.
    received_part_count: u32,
    /// A Vec with the received data of the parts at the correct index.
    /// The Vec is preallocated to have the length of the total part count.
    parts: Vec<Option<Box<[u8]>>>,
} 

impl SplitPacketChannel {
    pub fn new(reliability: InternalReliability, ordering: InternalOrdering, split_packet_count: u32) -> Self {
        SplitPacketChannel {
            reliability,
            ordering,
            received_byte_count: 0,
            received_part_count: 0,
            parts: vec![None; split_packet_count as usize],
        }
    }

    pub fn insert(&mut self, index: u32, data: Box<[u8]>) -> Result<()> {
        if index >= self.parts.len() as u32 {
            return Err(ReadError::SplitPacketIndexOutOfRange.into());
        }

        if self.parts[index as usize] != None {
            return Err(ReadError::DuplicateSplitPacketIndex.into());
        }

        self.received_byte_count = self.received_byte_count + data.len() as u32;
        self.received_part_count = self.received_part_count + 1;
        self.parts[index as usize] = Some(data);
        Ok(())
    }

    pub fn get_reassembled_packet(&self, time: Instant) -> Option<InternalPacket> {
        if self.has_complete_packet() {
            let mut payload = Vec::with_capacity(self.received_byte_count as usize);
            for part in self.parts.iter() {
                if let Some(data) = part {
                    payload.extend_from_slice(&data);
                } else {
                    error!("Missing split packet part even though packet should be complete");
                    return None;
                }
            }
            Some(InternalPacket::new(time, self.reliability, self.ordering, None, None, payload.into_boxed_slice()))
        } else {
            None
        }
    }

    fn has_complete_packet(&self) -> bool {
        self.received_part_count == self.parts.len() as u32
    }
}

pub struct SplitPacketHandler {
    channels: HashMap<u16, SplitPacketChannel>,
}

impl SplitPacketHandler {
    pub fn new() -> SplitPacketHandler {
        SplitPacketHandler {            
            channels: HashMap::with_capacity(10),
        }
    }

    pub fn handle_split_packet(&mut self, time: Instant, packet: InternalPacket) -> Option<InternalPacket> {
        if let Some(header) = packet.split_packet_header() {
            debug!("Split packet. count={}, id={}, idx={}", header.split_packet_count(), header.split_packet_id(), header.split_packet_index());

            let id = header.split_packet_id();

            if !self.channels.contains_key(&id) {
                self.channels.insert(id, SplitPacketChannel::new(packet.reliability(), packet.ordering(), header.split_packet_count()));
            }
    
            if let Some(channel) = self.channels.get_mut(&id) {
                if let Err(err) = channel.insert(header.split_packet_index(), packet.into_payload()) {
                    error!("Failed inserting split packet: {:?}", err);
                    return None;
                }

                // TODO: Send progress to user

                if let Some(packet) = channel.get_reassembled_packet(time) {
                    self.channels.remove(&id);
                    return Some(packet);
                }
            }
        }
        None
    }
}