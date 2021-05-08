use std::{net::SocketAddr, time::Instant};
use log::{debug, error};

use crate::{
    communicator::Communicator,
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
                // TODO: Schedule ACK on the received datagram to be sent next update (if it is time to send ACKs then)
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