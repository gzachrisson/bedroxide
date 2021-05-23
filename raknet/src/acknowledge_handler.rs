use std::{collections::HashMap, time::Instant};
use log::debug;

use crate::{error::Result, internal_packet::InternalPacket, number::DatagramSequenceNumber, packet_datagram::PacketDatagram};

struct DatagramItem {
    pub time_sent: Instant,
    pub packets: Vec<InternalPacket>,
}

pub struct AcknowledgeHandler {
    datagrams: HashMap<DatagramSequenceNumber, DatagramItem>,
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

    pub fn process_outgoing_datagram(&mut self, datagram: PacketDatagram, time: Instant, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        debug!("Processing datagram: {:?}", datagram);
        datagram.write(buf)?;
        debug!("Datagram bytes: {:?}", crate::utils::to_hex(&buf, 100));
        self.datagrams.insert(self.next_datagram_number, DatagramItem { time_sent: time, packets: datagram.into_packets() });
        self.next_datagram_number = self.next_datagram_number.wrapping_add(DatagramSequenceNumber::ONE);
        Ok(())
    }

    pub fn has_room_for_datagram(&self) -> bool {
        !self.datagrams.contains_key(&self.next_datagram_number)
    }

    pub fn datagrams_in_flight(&self) -> usize {
        self.datagrams.len()
    }
}