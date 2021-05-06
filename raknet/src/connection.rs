use std::{net::SocketAddr, time::Instant};
use log::{debug, error};

use crate::{
    communicator::Communicator,
    datagram_header::DatagramHeader,
    split_packet_handler::SplitPacketHandler,
    internal_packet::{InternalPacket, Ordering, Reliability},
    reader::{DataRead, DataReader},
    socket::DatagramSocket
};

pub struct Connection {
    connection_time: Instant,
    split_packet_handler: SplitPacketHandler,
    remote_addr: SocketAddr,
    guid: u64,
    is_incoming: bool,
    mtu: u16,
    pub state: ConnectionState,
}

impl Connection {
    pub fn incoming(connection_time: Instant, remote_addr: SocketAddr, guid: u64, mtu: u16) -> Connection {
        Connection {
            connection_time,
            split_packet_handler: SplitPacketHandler::new(),
            remote_addr,
            guid,
            is_incoming: true,
            mtu,
            state: ConnectionState::UnverifiedSender,
        }
    }

    /// Returns the GUID of the remote peer.
    pub fn guid(&self) -> u64 {
        self.guid
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
    pub fn process_incoming_datagram(&mut self, payload: &[u8], time: Instant, _communicator: &mut Communicator<impl DatagramSocket>) {
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
                while reader.has_more() {
                    match InternalPacket::read(time, &mut reader) {
                        Ok(mut packet) => {
                            debug!("Received a packet:\n{:?}", packet);
                            if let Reliability::Reliable(reliable_message_number) = packet.reliability() {
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
                                Ordering::None => {
                                    debug!("Packet is Unordered");
                                    // TODO: Deliver packet to user.
                                },
                                Ordering::Ordered { ordering_index, ordering_channel_index } => {
                                    debug!("Packed is Ordered. ord_idx={}, ord_ch_idx={}", ordering_index, ordering_channel_index);
                                    // TODO: Check if the order is correct so the packet can be delivered to the user. Buffer it otherwise.
                                    // TODO: Deliver packet to user.
                                },
                                Ordering::Sequenced { sequencing_index, ordering_index, ordering_channel_index } => {
                                    debug!("Packet id Reliable Sequenced. seq_idx={}, ord_idx={}, ord_ch_idx={}", sequencing_index, ordering_index, ordering_channel_index);
                                    // TODO: Check if the sequence is correct so the packet can be delivered to the user. Drop it or buffer it otherwise.
                                    // TODO: Deliver packet to user.
                                },
                            }
                        },
                        Err(err) => {
                            error!("Error reading packet: {:?}", err);
                            break;
                        }
                    }
                }
            },
            Err(err) => error!("Error parsing datagram header: {:?}", err),
        };

    }

    /// Returns true if this connection should be dropped.
    pub fn should_drop(&self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) -> bool {
        if self.state == ConnectionState::UnverifiedSender && time.saturating_duration_since(self.connection_time).as_millis() > communicator.config().incoming_connection_timeout_in_ms {
            debug!("Dropping connection from {} with guid {} because of connection timeout.", self.remote_addr, self.guid);
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