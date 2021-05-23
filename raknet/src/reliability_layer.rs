use std::{net::SocketAddr, time::Instant};
use log::{debug, error};

use crate::{
    acknowledge_handler::AcknowledgeHandler,
    communicator::Communicator,
    config::Config,
    constants::{MAX_ACK_DATAGRAM_HEADER_SIZE, MAX_NACK_DATAGRAM_HEADER_SIZE, NUMBER_OF_ORDERING_CHANNELS},
    datagram_header::DatagramHeader,
    datagram_range_list::DatagramRangeList,
    error::Result,
    internal_packet::{InternalOrdering, InternalPacket, InternalReliability, SplitPacketHeader}, 
    nack::OutgoingNacks,
    number::{OrderingChannelIndex, OrderingIndex, SequencingIndex},
    ordering_system::OrderingSystem,
    outgoing_acknowledgements::OutgoingAcknowledgements,
    outgoing_packet_heap::OutgoingPacketHeap,
    packet::{Ordering, Packet, Priority, Reliability},
    packet_datagram::PacketDatagram,
    reader::{DataRead, DataReader},
    reliable_message_number_handler::ReliableMessageNumberHandler,
    socket::DatagramSocket,
    split_packet_handler::SplitPacketHandler
};

pub struct ReliabilityLayer {
    acknowledge_handler: AcknowledgeHandler,
    outgoing_acks: OutgoingAcknowledgements,
    outgoing_nacks: OutgoingNacks,
    outgoing_packet_heap: OutgoingPacketHeap,
    reliable_message_number_handler: ReliableMessageNumberHandler,
    ordering_system: OrderingSystem,
    split_packet_handler: SplitPacketHandler,
    remote_addr: SocketAddr,
    remote_guid: u64,
    mtu: u16,
    time_last_datagram_arrived: Instant,
    next_ordering_index: [OrderingIndex; NUMBER_OF_ORDERING_CHANNELS as usize],
    next_sequencing_index: [SequencingIndex; NUMBER_OF_ORDERING_CHANNELS as usize],
    send_buffer: Vec<u8>,
}

impl ReliabilityLayer {
    pub fn new(remote_addr: SocketAddr, remote_guid: u64, mtu: u16) -> Self {
        ReliabilityLayer {
            acknowledge_handler: AcknowledgeHandler::new(),
            outgoing_acks: OutgoingAcknowledgements::new(),
            outgoing_nacks: OutgoingNacks::new(),
            outgoing_packet_heap: OutgoingPacketHeap::new(),
            reliable_message_number_handler: ReliableMessageNumberHandler::new(),
            ordering_system: OrderingSystem::new(),
            split_packet_handler: SplitPacketHandler::new(),
            remote_addr,
            remote_guid,
            mtu,
            time_last_datagram_arrived: Instant::now(),
            next_ordering_index: [OrderingIndex::ZERO; NUMBER_OF_ORDERING_CHANNELS as usize],
            next_sequencing_index: [SequencingIndex::ZERO; NUMBER_OF_ORDERING_CHANNELS as usize],
            send_buffer: Vec::new(),
        }
    }

    /// Processes an incoming datagram.
    pub fn process_incoming_datagram(&mut self, payload: &[u8], time: Instant, _communicator: &mut Communicator<impl DatagramSocket>) -> Option<Vec<Packet>> {
        self.time_last_datagram_arrived = time;
        let mut reader = DataReader::new(payload);
        match DatagramHeader::read(&mut reader) {
            Ok(DatagramHeader::Ack { data_arrival_rate }) => {
                debug!("Received ACK. data_arrival_rate={:?}", data_arrival_rate);
                // TODO: Send ACK receipt to user for unreliable packets with ACK receipt requested (and remove packets from list)
                // TODO: Remove ACK:ed packets from resend list (and send ACK receipt to user)
            },
            Ok(DatagramHeader::Nack) => {
                debug!("Received NACK");
                // TODO: Resend NACK:ed datagrams (by setting the next resend time to current time so they will be sent in next update)
            },
            Ok(DatagramHeader::Packet {is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }) => {
                debug!("Received a datagram of packets. is_packet_pair={}, is_continuous_send={}, needs_data_arrival_rate={}, datagram_number={}", 
                is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number);
                self.outgoing_nacks.handle_datagram(datagram_number);
                self.outgoing_acks.handle_datagram(datagram_number, time);

                match self.process_incoming_packets(reader, time) {
                    Ok(packets) => return Some(packets),
                    Err(err) => error!("Error reading packets: {:?}", err),
                }
            },
            Err(err) => error!("Error parsing datagram header: {:?}", err),
        };
        None
    }

    pub fn is_ack_timeout(&self, time: Instant, config: &Config) -> bool {
        self.acknowledge_handler.datagrams_in_flight() > 0 &&
            time.saturating_duration_since(self.time_last_datagram_arrived).as_millis() > config.ack_timeout_in_ms
    }

    pub fn update(&mut self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) {
        if self.is_ack_timeout(time, communicator.config()) {
            // Connection is dead and will be dropped
            return;
        }
        
        if self.outgoing_acks.should_send_acks(time) {
            self.send_acks(communicator);
        }

        if !self.outgoing_nacks.is_empty() {
            self.send_nacks(communicator);
        }
        
        // TODO: Resend not ACKed packets
        // TODO: Send outgoing packets        
        let mut datagram = PacketDatagram::new(self.acknowledge_handler.get_next_datagram_number());
        loop {
            while let Some(packet) = self.outgoing_packet_heap.peek() {
                if !datagram.has_room_for(packet, self.mtu) {
                    // Datagram full, break out of loop and send datagram
                    break;
                }
                if let Some(mut packet) = self.outgoing_packet_heap.pop() {
                    if let InternalReliability::Reliable(None) = packet.reliability() {
                        let realiable_message_number = self.reliable_message_number_handler.get_and_increment_reliable_message_number();
                        packet.set_reliability(InternalReliability::Reliable(Some(realiable_message_number)));
                    }
                    datagram.push(packet);
                }
            }
            if datagram.is_empty() {
                // Nothing more to send, break out of loop
                break;
            }
            match self.acknowledge_handler.process_outgoing_datagram(datagram, &mut self.send_buffer) {
                Ok(()) => {
                    communicator.send_datagram(&self.send_buffer, self.remote_addr);
                    datagram = PacketDatagram::new(self.acknowledge_handler.get_next_datagram_number());
                },
                Err(err) => {
                    error!("Failed processing outgoing datagram: {:?}", err);
                    break;
                }
            }
        }

    }

    /// Enqueues a packet for sending.
    pub fn send_packet(&mut self, time: Instant, priority: Priority, reliability: Reliability, ordering: Ordering, receipt: Option<u32>, payload: Box<[u8]>) {
        // TODO: Store the time when the last reliable send was done (if reliable)
        // TODO: Enqueue packet for sending
        if payload.len() > self.get_max_packet_payload_size() as usize {
            // TODO: Split packet
            // TODO: Set reliability to Reliability::Reliable for split packet if unreliable.
        } else {
            self.send_packet_internal(time, priority, reliability, ordering, None, receipt, payload);
        }
    }

    /// Enqueues a packet that is expected to have been pre-split into parts that can fit a datagram.
    fn send_packet_internal(&mut self, time: Instant, priority: Priority, reliability: Reliability,
        ordering: Ordering, split_packet_header: Option<SplitPacketHeader>, receipt: Option<u32>, payload: Box<[u8]>) {
        // TODO: Store the time when the last reliable send was done (if reliable)
        let reliability = match reliability {
            Reliability::Unreliable => InternalReliability::Unreliable,
            Reliability::Reliable => InternalReliability::Reliable(None),
        };
        let ordering = match ordering {
            Ordering::None => InternalOrdering::None,
            Ordering::Ordered(ordering_channel_index) => {
                let ordering_channel_index = if ordering_channel_index < NUMBER_OF_ORDERING_CHANNELS { ordering_channel_index } else { 0 };
                let ordering_index = self.get_and_increment_ordering_index(ordering_channel_index);
                self.clear_sequencing_index(ordering_channel_index);
                InternalOrdering::Ordered {
                    ordering_index,
                    ordering_channel_index,
                }
            },
            Ordering::Sequenced(ordering_channel_index) => {
                let ordering_channel_index = if ordering_channel_index < NUMBER_OF_ORDERING_CHANNELS { ordering_channel_index } else { 0 };
                InternalOrdering::Sequenced {
                    sequencing_index: self.get_and_increment_sequencing_index(ordering_channel_index),
                    ordering_index: self.get_ordering_index(ordering_channel_index),
                    ordering_channel_index,
                }
            },
        };
        let packet = InternalPacket::new(time, reliability, ordering, split_packet_header, receipt, payload);
        self.outgoing_packet_heap.push(priority, packet);
    }

    fn clear_sequencing_index(&mut self, ordering_channel_index: OrderingChannelIndex) {
        self.next_sequencing_index[ordering_channel_index as usize] = SequencingIndex::ZERO;
    }

    fn get_ordering_index(&self, ordering_channel_index: OrderingChannelIndex) -> OrderingIndex {
        self.next_ordering_index[ordering_channel_index as usize]
    }

    fn get_and_increment_ordering_index(&mut self, ordering_channel_index: OrderingChannelIndex) -> OrderingIndex {
        let ordering_index = self.next_ordering_index[ordering_channel_index as usize];
        self.next_ordering_index[ordering_channel_index as usize] = ordering_index.wrapping_add(OrderingIndex::ONE);
        ordering_index
    }

    fn get_and_increment_sequencing_index(&mut self, ordering_channel_index: OrderingChannelIndex) -> SequencingIndex {
        let sequencing_index = self.next_sequencing_index[ordering_channel_index as usize];
        self.next_sequencing_index[ordering_channel_index as usize] = sequencing_index.wrapping_add(SequencingIndex::ONE);
        sequencing_index
    }    

    fn get_max_packet_payload_size(&self) -> u16 {
        // Bitflags (u8) + data bit length (u16) + reliable message number (u24)
        // + seuencing index (u24) + ordering index (u24) + ordering channel (u8)
        // + split packet count (u32) + split packet ID (u16) + split packet index (u32)        
        let max_packet_header_size = 1 + 2 + 3 + 3 + 3 + 1 + 4 + 2 + 4;
        PacketDatagram::get_max_payload_size(self.mtu) - max_packet_header_size
    }

    /// Sends all waiting outgoing acknowledgements.
    fn send_acks(&mut self, communicator: &mut Communicator<impl DatagramSocket>) {
        // TODO: Check calculation (MTU - datagram header (bitflags: u8=1, AS: f32=4))
        let max_datagram_payload = self.mtu as usize - MAX_ACK_DATAGRAM_HEADER_SIZE;
        while !self.outgoing_acks.is_empty() {
            let mut ack_range_list = DatagramRangeList::new();
            while !ack_range_list.is_full(max_datagram_payload) {
                if let Some(range) = self.outgoing_acks.pop_range() {
                    ack_range_list.push(range);
                } else {
                    // No more ranges                    
                    break;
                }
            }

            let datagram_header = DatagramHeader::Ack { data_arrival_rate: None };
            let mut buf = Vec::with_capacity(MAX_ACK_DATAGRAM_HEADER_SIZE + ack_range_list.bytes_used());
            if let Err(err) = datagram_header.write(&mut buf) {
                error!("Could not write datagram header: {:?}", err);
                continue;
            }
            if let Err(err) = ack_range_list.write(&mut buf) {
                error!("Could not write ACKs payload: {:?}", err);
                continue;
            }

            debug!("Sending ACKs: {:?}", ack_range_list);
            communicator.send_datagram(&buf, self.remote_addr);
        }
    }

    /// Sends all waiting outgoing NACKs.
    fn send_nacks(&mut self, communicator: &mut Communicator<impl DatagramSocket>) {
        // TODO: Check calculation (MTU - datagram header (bitflags: u8=1))
        let max_datagram_payload = self.mtu as usize - MAX_NACK_DATAGRAM_HEADER_SIZE;
        while !self.outgoing_nacks.is_empty() {
            let mut nack_range_list = DatagramRangeList::new();
            while !nack_range_list.is_full(max_datagram_payload) {
                if let Some(range) = self.outgoing_nacks.pop_range() {
                    nack_range_list.push(range);
                } else {
                    // No more ranges                    
                    break;
                }
            }

            let datagram_header = DatagramHeader::Nack;
            let mut buf = Vec::with_capacity(MAX_NACK_DATAGRAM_HEADER_SIZE + nack_range_list.bytes_used());
            if let Err(err) = datagram_header.write(&mut buf) {
                error!("Could not write datagram header: {:?}", err);
                continue;
            }
            if let Err(err) = nack_range_list.write(&mut buf) {
                error!("Could not write NACKs payload: {:?}", err);
                continue;
            }

            debug!("Sending NACKs: {:?}", nack_range_list);
            communicator.send_datagram(&buf, self.remote_addr);
        }
    }    

    /// Processes all incoming packets contained in a a datagram after the datagram header
    /// has been read.
    fn process_incoming_packets(&mut self, mut reader: DataReader, time: Instant) -> Result<Vec<Packet>> {
        let mut packets = Vec::new();
        while reader.has_more() {
            let mut packet = InternalPacket::read(time, &mut reader)?;
            debug!("Received a packet:\n{:?}", packet);
            if let InternalReliability::Reliable(Some(reliable_message_number)) = packet.reliability() {
                debug!("Packet is reliable with message number {}", reliable_message_number);
                if self.reliable_message_number_handler.should_discard_packet(reliable_message_number) {
                    debug!("Dropping packet with duplicate message number: {}", reliable_message_number);
                    continue;
                }
            }

            if let Some(header) = packet.split_packet_header() {
                if let Some(defragmented_packet) = self.split_packet_handler.handle_split_packet(header, packet) {
                    packet = defragmented_packet;
                } else {
                    continue;
                }
            }

            match packet.ordering() {
                InternalOrdering::None => {
                    debug!("Packet is Unordered");
                    packets.push(Packet::new(self.remote_addr, self.remote_guid, packet.into_payload()));
                },
                InternalOrdering::Ordered { ordering_index, ordering_channel_index } => {
                    debug!("Packed is Ordered. ord_idx={}, ord_ch_idx={}", ordering_index, ordering_channel_index);
                    if let Some(ordering_channel) = self.ordering_system.get_channel(ordering_channel_index) {
                        let addr = self.remote_addr;
                        let guid = self.remote_guid;
                        packets.extend(ordering_channel
                            .process_incoming(None, ordering_index, packet.into_payload())
                            .into_iter()
                            .chain(ordering_channel.iter_mut())
                            .map(|payload| Packet::new(addr, guid, payload))
                        );
                    } else {
                        error!("Invalid ordering channel: {}", ordering_channel_index);
                    }
                },
                InternalOrdering::Sequenced { sequencing_index, ordering_index, ordering_channel_index } => {
                    debug!("Packet id Reliable Sequenced. seq_idx={}, ord_idx={}, ord_ch_idx={}", sequencing_index, ordering_index, ordering_channel_index);
                    if let Some(ordering_channel) = self.ordering_system.get_channel(ordering_channel_index) {
                        if let Some(payload) = ordering_channel.process_incoming(Some(sequencing_index), ordering_index, packet.into_payload()) {
                            packets.push(Packet::new(self.remote_addr, self.remote_guid, payload));
                        }
                    } else {
                        error!("Invalid ordering channel: {}", ordering_channel_index);
                    }
                },
            }
        }
        Ok(packets)
    }
}