use std::{net::SocketAddr, io::Cursor, time::Instant};
use log::{debug, error};

use crate::{communicator::Communicator, datagram_header::DatagramHeader, socket::DatagramSocket};

pub struct Connection {
    connection_time: Instant,
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

    /// Processes an incoming package.
    pub fn process_incoming_packet(&mut self, payload: &[u8], _time: Instant, _communicator: &mut Communicator<impl DatagramSocket>) {
        // TODO: Implement

        let mut reader = Cursor::new(payload);
        match DatagramHeader::read(&mut reader) {
            Ok(DatagramHeader::Ack { data_arrival_rate }) => {
                debug!("Received ACK. data_arrival_rate={:?}", data_arrival_rate);
            },
            Ok(DatagramHeader::Nack) => {
                debug!("Received NACK");
            },
            Ok(DatagramHeader::Packet {is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }) => {
                debug!("Received a datagram of packets. is_packet_pair={}, is_continuous_send={}, needs_data_arrival_rate={}, datagram_number={}", 
                is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number);
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