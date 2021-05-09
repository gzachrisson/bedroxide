use std::{net::SocketAddr, time::Instant};
use log::{debug, error};

use crate::{
    acknowledgement::{Acknowledgement, OutgoingAcknowledgements},
    communicator::Communicator,
    constants::MAX_ACK_DATAGRAM_HEADER_SIZE,
    datagram_header::DatagramHeader,
    split_packet_handler::SplitPacketHandler,
    internal_packet::{InternalPacket, InternalOrdering, InternalReliability},
    reader::{DataRead, DataReader},
    Packet,
    PeerEvent,
    Result,
    socket::DatagramSocket
};

pub struct Connection {
    outgoing_acks: OutgoingAcknowledgements,
    connection_time: Instant,
    split_packet_handler: SplitPacketHandler,
    remote_addr: SocketAddr,
    remote_guid: u64,
    is_incoming: bool,
    mtu: u16,
    pub state: ConnectionState,
}

impl Connection {
    pub fn incoming(connection_time: Instant, remote_addr: SocketAddr, remote_guid: u64, mtu: u16) -> Connection {
        Connection {
            outgoing_acks: OutgoingAcknowledgements::new(),
            connection_time,
            split_packet_handler: SplitPacketHandler::new(),
            remote_addr,
            remote_guid,
            is_incoming: true,
            mtu,
            state: ConnectionState::UnverifiedSender,
        }
    }

    /// Returns the GUID of the remote peer.
    pub fn guid(&self) -> u64 {
        self.remote_guid
    }

    /// Returns the agreed MTU for this connection.
    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    /// Returns true if the connection was initiated
    /// by a remote peer.
    pub fn is_incoming(&self) -> bool {
        self.is_incoming
    }

    /// Performs various connection related actions such as sending acknowledgements
    /// and resending dropped packets.
    pub fn update(&mut self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) {
        if self.outgoing_acks.should_send_acks(time) {
            self.send_acks(communicator);
        }
    }

    /// Sends all waiting outgoing acknowledgements.
    fn send_acks(&mut self, communicator: &mut Communicator<impl DatagramSocket>) {
        // TODO: Check calculation (MTU - datagram header (bitflags: u8=1, AS: f32=4))
        let max_datagram_payload = self.mtu as usize - MAX_ACK_DATAGRAM_HEADER_SIZE;
        while !self.outgoing_acks.is_empty() {
            let mut ack = Acknowledgement::new();
            while !ack.is_full(max_datagram_payload) {
                if let Some(range) = self.outgoing_acks.pop_range() {
                    ack.push(range);
                } else {
                    // No more ack ranges                    
                    break;
                }
            }

            let datagram_header = DatagramHeader::Ack { data_arrival_rate: None };
            let mut buf = Vec::with_capacity(MAX_ACK_DATAGRAM_HEADER_SIZE + ack.bytes_used());
            if let Err(err) = datagram_header.write(&mut buf) {
                error!("Could not write datagram header: {:?}", err);
                continue;
            }
            if let Err(err) = ack.write(&mut buf) {
                error!("Could not write ack payload: {:?}", err);
                continue;
            }

            debug!("Sending ack: {:?}", ack);
            communicator.send_datagram(&buf, self.remote_addr);
        }
    }

    /// Processes an incoming datagram.
    pub fn process_incoming_datagram(&mut self, payload: &[u8], time: Instant, communicator: &mut Communicator<impl DatagramSocket>) {
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
                // TODO: Schedule NACKs on missed datagrams to be sent next update
                
                self.outgoing_acks.insert(datagram_number, time);

                match self.process_incoming_packets(reader, time) {
                    Ok(packets) => {
                        for packet in packets.into_iter() {
                            // TODO: Filter out connection related packets and act on them
                            communicator.send_event(PeerEvent::Packet(packet));
                        }
                    }
                    Err(err) => error!("Error reading packets: {:?}", err),
                }
            },
            Err(err) => error!("Error parsing datagram header: {:?}", err),
        };
    }

    /// Processes all incoming packets contained in a a datagram after the datagram header
    /// has been read.
    fn process_incoming_packets(&mut self, mut reader: DataReader, time: Instant) -> Result<Vec<Packet>> {
        let mut packets = Vec::new();
        while reader.has_more() {
            let mut packet = InternalPacket::read(time, &mut reader)?;
            debug!("Received a packet:\n{:?}", packet);
            if let InternalReliability::Reliable(reliable_message_number) = packet.reliability() {
                debug!("Packet is reliable with message number {}", reliable_message_number);
                // TODO: Check if the reliable message number is the expected. Update holes and the expected. Drop if it is a duplicate.
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
                    // TODO: Check if the order is correct so the packet can be delivered to the user. Buffer it otherwise.
                    packets.push(Packet::new(self.remote_addr, self.remote_guid, packet.into_payload()));
                },
                InternalOrdering::Sequenced { sequencing_index, ordering_index, ordering_channel_index } => {
                    debug!("Packet id Reliable Sequenced. seq_idx={}, ord_idx={}, ord_ch_idx={}", sequencing_index, ordering_index, ordering_channel_index);
                    // TODO: Check if the sequence is correct so the packet can be delivered to the user. Drop it or buffer it otherwise.
                    packets.push(Packet::new(self.remote_addr, self.remote_guid, packet.into_payload()));
                },
            }
        }
        Ok(packets)
    }

    /// Returns true if this connection should be dropped.
    pub fn should_drop(&self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) -> bool {
        // TODO: Add more conditions and in some scenarios send a packet to the remote peer.
        if self.state == ConnectionState::UnverifiedSender && time.saturating_duration_since(self.connection_time).as_millis() > communicator.config().incoming_connection_timeout_in_ms {
            debug!("Dropping connection from {} with guid {} because of connection timeout.", self.remote_addr, self.remote_guid);
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ConnectionState {
    UnverifiedSender,
    Connected,
}