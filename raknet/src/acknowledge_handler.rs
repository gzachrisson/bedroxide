use std::{collections::HashMap, net::SocketAddr, time::{Duration, Instant}};
use log::debug;

use crate::{
    communicator::Communicator,
    datagram_range_list::DatagramRangeList,
    socket::DatagramSocket,
    error::Result,
    internal_packet::{InternalPacket, InternalReliability},
    number::DatagramSequenceNumber,
    packet_datagram::PacketDatagram,
    peer_event::PeerEvent,
    send_receipt::SendReceipt,
};

struct DatagramItem {
    pub timeout_time: Instant,
    pub packets: Vec<InternalPacket>,
}

pub struct AcknowledgeHandler {
    datagrams: HashMap<DatagramSequenceNumber, DatagramItem>,
    next_datagram_number: DatagramSequenceNumber,
    remote_addr: SocketAddr,
    remote_guid: u64,    
}

impl AcknowledgeHandler {
    pub fn new(remote_addr: SocketAddr, remote_guid: u64,) -> Self {
        AcknowledgeHandler {
            datagrams: HashMap::new(),
            next_datagram_number: DatagramSequenceNumber::ZERO,
            remote_addr,
            remote_guid,
        }
    }

    pub fn get_next_datagram_number(&self) -> DatagramSequenceNumber {
        self.next_datagram_number
    }

    pub fn get_packets_to_resend(&mut self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) -> Vec<InternalPacket> {
        let timed_out_datagram_numbers: Vec<DatagramSequenceNumber> = self.datagrams.iter().filter_map(|(number, datagram)|
            if time >= datagram.timeout_time || self.get_next_datagram_number().wrapping_less_than(*number) {
                Some(*number)
            } else {
                None
            }
        ).collect();

        let remote_addr = self.remote_addr;
        let remote_guid = self.remote_guid;
        timed_out_datagram_numbers.iter().filter_map(|number| {
            if let Some(datagram) = self.datagrams.remove(number) {
                Some(datagram.packets)
            } else {
                None
            }
        }).flatten().filter(|packet| if let InternalReliability::Unreliable = packet.reliability() {
            if let Some(receipt) = packet.receipt() {
                communicator.send_event(PeerEvent::SendReceiptLoss(SendReceipt::new(remote_addr, remote_guid, receipt)));
            }
            false
        } else {
            true
        })
        .collect()
    }

    pub fn process_outgoing_datagram(&mut self, datagram: PacketDatagram, time: Instant, buf: &mut Vec<u8>) -> Result<()> {
        buf.clear();
        datagram.write(buf)?;
        let timeout_time = time + Self::get_retransmission_timeout();
        self.datagrams.insert(self.next_datagram_number, DatagramItem { timeout_time, packets: datagram.into_packets() });
        self.next_datagram_number = self.next_datagram_number.wrapping_add(DatagramSequenceNumber::ONE);
        Ok(())
    }
    
    pub fn process_incoming_ack(&mut self, datagram_range_list: DatagramRangeList, communicator: &mut Communicator<impl DatagramSocket>) {
        for range in datagram_range_list.into_vec() {
            let mut number = range.start();
            while number.wrapping_less_than(range.end()) || number == range.end() {
                if let Some(datagram) = self.datagrams.remove(&number) {
                    for packet in datagram.packets {
                        if let Some(receipt) = packet.receipt() {
                            communicator.send_event(PeerEvent::SendReceiptAcked(SendReceipt::new(self.remote_addr, self.remote_guid, receipt)));
                        }
                    }
                } else {
                    debug!("Received ACK for unknown datagram {}", number);
                }
                number = number.wrapping_add(DatagramSequenceNumber::ONE);
            }
        }        
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