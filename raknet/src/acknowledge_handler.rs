use std::collections::HashMap;
use log::debug;

use crate::{error::Result, internal_packet::InternalPacket, number::DatagramSequenceNumber, packet_datagram::PacketDatagram};

pub struct AcknowledgeHandler {
    datagrams: HashMap<DatagramSequenceNumber, Vec<InternalPacket>>,
    next_datagram_number: DatagramSequenceNumber,
}

impl AcknowledgeHandler {
    pub fn new() -> Self {
        AcknowledgeHandler {
            datagrams: HashMap::new(),
            next_datagram_number: DatagramSequenceNumber::ZERO,
        }
    }

    pub fn get_next_datagram_number(&self) -> DatagramSequenceNumber {
        self.next_datagram_number
    }

    pub fn process_outgoing_datagram(&mut self, datagram: PacketDatagram, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        debug!("Processing datagram: {:?}", datagram);
        datagram.write(buf)?;
        debug!("Datagram bytes: {:?}", crate::utils::to_hex(&buf, 100));
        self.datagrams.insert(self.next_datagram_number, datagram.into_packets());
        self.next_datagram_number = self.next_datagram_number.wrapping_add(DatagramSequenceNumber::ONE);
        Ok(())
    }

    pub fn datagrams_in_flight(&self) -> usize {
        self.datagrams.len()
    }
}