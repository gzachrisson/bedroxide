use std::{collections::HashMap, time::{Duration, Instant}};
use log::debug;

use crate::{
    datagram_range_list::DatagramRangeList,
    error::Result,
    internal_packet::InternalPacket,
    number::DatagramSequenceNumber,
    packet_datagram::PacketDatagram
};

struct DatagramItem {
    pub timeout_time: Instant,
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
        let timeout_time = time + Self::get_retransmission_timeout();
        self.datagrams.insert(self.next_datagram_number, DatagramItem { timeout_time, packets: datagram.into_packets() });
        self.next_datagram_number = self.next_datagram_number.wrapping_add(DatagramSequenceNumber::ONE);
        Ok(())
    }
    
    pub fn process_incoming_nack(&mut self, time: Instant, datagram_range_list: DatagramRangeList) {
        for range in datagram_range_list.into_vec() {
            let mut number = range.start();
            while number.wrapping_less_than(range.end()) || number == range.end() {
                if let Some(datagram) = self.datagrams.get_mut(&number) {
                    // Resend packets in NACK:ed datagram by setting the timeout_time to current time
                    datagram.timeout_time = time;
                }
                number = number.wrapping_add(DatagramSequenceNumber::ONE);
            }
        }
    }

    /// Returns the retransmission timeout (RTO) duration which is the time
    /// from that a packet is sent until it should be resent if no ACK
    /// has been received.
    pub fn get_retransmission_timeout() -> Duration {
        // TODO: Calculate retransmission timeout from the round-trip time (RTT) to reduce the delay
        Duration::from_millis(1000)
    }

    pub fn has_room_for_datagram(&self) -> bool {
        !self.datagrams.contains_key(&self.next_datagram_number)
    }

    pub fn datagrams_in_flight(&self) -> usize {
        self.datagrams.len()
    }
}