use std::{collections::{HashMap, VecDeque}, net::SocketAddr, time::{Duration, Instant}};
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

#[derive(Debug)]
struct DatagramItem {
    pub timeout_time: Instant,
    pub packets: Vec<InternalPacket>,
}

#[derive(Debug)]
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

    pub fn get_packets_to_resend(&mut self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) -> VecDeque<InternalPacket> {
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

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::{Duration, Instant}};   
    use crossbeam_channel::{Sender, Receiver, unbounded};
    use crate::{
        communicator::Communicator,
        config::Config,
        internal_packet::{InternalOrdering, InternalPacket, InternalReliability}, 
        number::{DatagramSequenceNumber, MessageNumber},
        packet_datagram::PacketDatagram,
        peer_event::PeerEvent,
        socket::FakeDatagramSocket
    };
    use super::AcknowledgeHandler;

    fn test_setup() -> (AcknowledgeHandler, Communicator<FakeDatagramSocket>, Sender<(Vec<u8>, SocketAddr)>, Receiver<(Vec<u8>, SocketAddr)>, Receiver<PeerEvent>, SocketAddr) {
        let local_addr = "127.0.0.2:19132".parse::<SocketAddr>().expect("Could not create address");
        let remote_addr =  "127.0.0.1:19132".parse::<SocketAddr>().expect("Could not create address");
        let remote_guid = 0x112233;
        let handler = AcknowledgeHandler::new(remote_addr, remote_guid);
        let fake_socket = FakeDatagramSocket::new(local_addr);
        let datagram_sender = fake_socket.get_datagram_sender();
        let datagram_receiver = fake_socket.get_datagram_receiver();
        let config = Config::default();
        let (event_sender, event_receiver) = unbounded();
        let communicator = Communicator::new(fake_socket, config, event_sender);
        (handler, communicator, datagram_sender, datagram_receiver, event_receiver, remote_addr)
    }

    #[test]
    fn get_packets_to_resend_no_packets() {
        // Arrange
        let (mut handler, mut communicator, mut _datagram_sender, mut _datagram_receiver, mut _event_receiver, _remote_addr) = test_setup(); 

        // Act
        let packets = handler.get_packets_to_resend(Instant::now(), &mut communicator);

        // Assert
        assert_eq!(packets, vec![]);
    }

    #[test]
    fn get_packets_to_resend_no_timeout() {
        // Arrange
        let (mut handler, mut communicator, mut _datagram_sender, mut _datagram_receiver, mut _event_receiver, _remote_addr) = test_setup();
        let time = Instant::now();
        let mut buf = Vec::new();
        let mut datagram1 = PacketDatagram::new(DatagramSequenceNumber::ZERO);
        datagram1.push(InternalPacket::new(time, InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1, 2, 3].into_boxed_slice()));
        datagram1.push(InternalPacket::new(time + Duration::from_millis(10), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1, 2, 3].into_boxed_slice()));
        handler.process_outgoing_datagram(datagram1, time, &mut buf).expect("Could not process datagram");
        let mut datagram2 = PacketDatagram::new(DatagramSequenceNumber::ZERO);
        datagram2.push(InternalPacket::new(time + Duration::from_millis(20), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1, 2, 3].into_boxed_slice()));
        datagram2.push(InternalPacket::new(time + Duration::from_millis(30), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1, 2, 3].into_boxed_slice()));
        handler.process_outgoing_datagram(datagram2, time, &mut buf).expect("Could not process datagram");

        // Act
        let packets = handler.get_packets_to_resend(time + Duration::from_millis(40), &mut communicator);

        // Assert
        assert_eq!(packets, vec![]);
    }

    #[test]
    fn get_packets_to_resend_timeout_unreliable() {
        // Arrange
        let (mut handler, mut communicator, mut _datagram_sender, mut _datagram_receiver, mut _event_receiver, _remote_addr) = test_setup();
        let time = Instant::now();
        let mut buf = Vec::new();
        let packet1 = InternalPacket::new(time, InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(time, InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());
        let packet3 = InternalPacket::new(time, InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![3].into_boxed_slice());
        let packet4 = InternalPacket::new(time, InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![4].into_boxed_slice());
        let mut datagram1 = PacketDatagram::new(DatagramSequenceNumber::ZERO);
        let mut datagram2 = PacketDatagram::new(DatagramSequenceNumber::ONE);
        let mut datagram3 = PacketDatagram::new(DatagramSequenceNumber::from_masked_u32(2));
        datagram1.push(packet1.clone());
        datagram1.push(packet2.clone());
        datagram2.push(packet3.clone());
        datagram3.push(packet4.clone());
        handler.process_outgoing_datagram(datagram1, time, &mut buf).expect("Could not process datagram");
        handler.process_outgoing_datagram(datagram2, time + Duration::from_millis(10), &mut buf).expect("Could not process datagram");
        handler.process_outgoing_datagram(datagram3, time + Duration::from_millis(30), &mut buf).expect("Could not process datagram");

        // Act
        let packets = handler.get_packets_to_resend(time + Duration::from_millis(1025), &mut communicator);

        // Assert
        assert_eq!(packets, vec![]);
    }

    #[test]
    fn get_packets_to_resend_timeout_reliable() {
        // Arrange
        let (mut handler, mut communicator, mut _datagram_sender, mut _datagram_receiver, mut _event_receiver, _remote_addr) = test_setup();
        let time = Instant::now();
        let mut buf = Vec::new();
        let packet1 = InternalPacket::new(time, InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(1))), InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(time, InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(2))), InternalOrdering::None, None, None, vec![2].into_boxed_slice());
        let packet3 = InternalPacket::new(time, InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(3))), InternalOrdering::None, None, None, vec![3].into_boxed_slice());
        let packet4 = InternalPacket::new(time, InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(4))), InternalOrdering::None, None, None, vec![4].into_boxed_slice());
        let mut datagram1 = PacketDatagram::new(DatagramSequenceNumber::ZERO);
        let mut datagram2 = PacketDatagram::new(DatagramSequenceNumber::ONE);
        let mut datagram3 = PacketDatagram::new(DatagramSequenceNumber::from_masked_u32(2));
        datagram1.push(packet1.clone());
        datagram1.push(packet2.clone());
        datagram2.push(packet3.clone());
        datagram3.push(packet4.clone());
        handler.process_outgoing_datagram(datagram1, time, &mut buf).expect("Could not process datagram");
        handler.process_outgoing_datagram(datagram2, time + Duration::from_millis(10), &mut buf).expect("Could not process datagram");
        handler.process_outgoing_datagram(datagram3, time + Duration::from_millis(30), &mut buf).expect("Could not process datagram");

        // Act
        let packets = handler.get_packets_to_resend(time + Duration::from_millis(1025), &mut communicator);

        // Assert
        assert_eq!(packets, vec![packet1, packet2, packet3]);
    }

}