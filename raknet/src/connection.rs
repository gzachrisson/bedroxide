use std::{convert::TryFrom, net::SocketAddr, time::Instant};
use log::{debug, error};

use crate::{
    communicator::Communicator,
    incoming_connection::IncomingConnection,
    message_ids::MessageId,
    messages::{ConnectedPingMessage, ConnectedPongMessage, ConnectionRequestMessage, ConnectionRequestAcceptedMessage, NewIncomingConnectionMessage},
    packet::{Ordering, Packet, Priority, Reliability},
    PeerEvent,
    reader::{DataReader, MessageRead},
    reliability_layer::ReliabilityLayer,
    socket::DatagramSocket,
    writer::MessageWrite
};

pub struct Connection {
    reliability_layer: ReliabilityLayer,
    connection_time: Instant,
    peer_creation_time: Instant,
    remote_addr: SocketAddr,
    remote_guid: u64,
    is_incoming: bool,
    mtu: u16,
    pub state: ConnectionState,
}

impl Connection {
    pub fn incoming(connection_time: Instant, peer_creation_time: Instant, remote_addr: SocketAddr, remote_guid: u64, mtu: u16) -> Connection {
        Connection {
            reliability_layer: ReliabilityLayer::new(remote_addr, remote_guid, mtu),
            connection_time,
            peer_creation_time,
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
        // TODO: Read outgoing packets from the user and send to the reliability layer
        // TODO: Send a connected ping if a reliable packet has not been sent within half the timeout time
        self.reliability_layer.update(time, communicator);
    }

    /// Processes an incoming datagram.
    pub fn process_incoming_datagram(&mut self, payload: &[u8], time: Instant, communicator: &mut Communicator<impl DatagramSocket>) {
        if let Some(packets) = self.reliability_layer.process_incoming_datagram(payload, time, communicator) {
            for packet in packets.into_iter() {
                if !self.handle_connection_related_packet(&packet, communicator, time) {
                    communicator.send_event(PeerEvent::Packet(packet));
                }
            }
        }
    }

    /// Handles connection related incoming packets.
    /// Returns true if the packet is handled and should not be delivered to the user.
    fn handle_connection_related_packet(&mut self, packet: &Packet, communicator: &mut Communicator<impl DatagramSocket>, time: Instant) -> bool {
        if packet.payload().len() == 0 {
            return true;
        }
        if self.state == ConnectionState::UnverifiedSender {
            match MessageId::try_from(packet.payload()[0]) {
                Ok(MessageId::ConnectionRequest) => self.handle_connection_request(packet.payload(), communicator, time),
                _ => {}, // TODO: Close the connection and ban the user temporarily for sending garbage
            }
        } else {
            match MessageId::try_from(packet.payload()[0]) {
                Ok(MessageId::ConnectionRequest) => {}, // TODO: Implement
                Ok(MessageId::NewIncomingConnection) => self.handle_new_incoming_connection(packet.payload(), communicator, time),
                Ok(MessageId::ConnectedPong) => {}, // TODO: Implement
                Ok(MessageId::ConnectedPing) => self.handle_connected_ping(packet.payload(), time),
                Ok(MessageId::DisconnectionNotification) => {}, // TODO: Implement
                Ok(MessageId::DetectLostConnections) => {}, // TODO: Implement
                Ok(MessageId::InvalidPassword) => {}, // TODO: Implement
                Ok(MessageId::ConnectionRequestAccepted) => {}, // TODO: Implement
                _ => return false,
            }
        }
        true
    }

    fn handle_connection_request(&mut self, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, time: Instant) {
        let mut reader = DataReader::new(payload);
        match ConnectionRequestMessage::read_message(&mut reader) {
            Ok(connection_request) => {
                debug!("Received a connection request: {:?}", connection_request);
                // TODO: Check proof, client key and password
                self.state = ConnectionState::HandlingConnectionRequest;
                let message = ConnectionRequestAcceptedMessage {
                    client_addr: self.remote_addr,
                    client_index: 0, // TODO: Fix this dummy value by increasing a counter for each created connection.
                    ip_list: communicator.get_addr_list(),
                    client_time: connection_request.time,
                    server_time: time.saturating_duration_since(self.peer_creation_time).as_millis() as u64,
                };
                self.send_connected_message(time, &message, Reliability::Reliable, Ordering::Ordered(0));
            },
            Err(err) => error!("Failed reading connection request message: {}", err),
        }
    }

    fn handle_new_incoming_connection(&mut self, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, time: Instant) {
        let mut reader = DataReader::new(payload);
        match NewIncomingConnectionMessage::read_message(&mut reader) {
            Ok(incoming_connection) => {
                debug!("Received a new incoming connection: {:?}", incoming_connection);
                if self.state == ConnectionState::HandlingConnectionRequest {
                    self.state = ConnectionState::Connected;
                    self.send_connected_ping(time);
                    communicator.send_event(PeerEvent::IncomingConnection(IncomingConnection::new(self.remote_addr, self.remote_guid)));
                    // TODO: Possibly store the received external IP and the client's internal IPs
                    // TODO: Store the ping and clock differential
                } else {
                    debug!("Already connected, ignoring packet");
                }
            },
            Err(err) => error!("Failed reading connection request message: {}", err),
        }
    }

    fn handle_connected_ping(&mut self, payload: &[u8], time: Instant) {
        let mut reader = DataReader::new(payload);
        match ConnectedPingMessage::read_message(&mut reader) {
            Ok(ping) => {
                let pong = ConnectedPongMessage { send_ping_time: ping.time, send_pong_time: self.get_peer_time(time) };
                self.send_connected_message(time, &pong, Reliability::Unreliable, Ordering::None);
            },
            Err(err) => error!("Failed reading connection request message: {}", err),
        }
    }

    fn send_connected_ping(&mut self, time: Instant) {
        let ping = ConnectedPingMessage { time: self.get_peer_time(time) };
        self.send_connected_message(time, &ping, Reliability::Unreliable, Ordering::None);
    }

    /// Returns the time in milliseconds since the `Peer` was created.
    fn get_peer_time(&self, time: Instant) -> u64 {
        time.saturating_duration_since(self.peer_creation_time).as_millis() as u64
    }

    fn send_connected_message(&mut self, time: Instant, message: &dyn MessageWrite, reliability: Reliability, ordering: Ordering) {
        let mut payload = Vec::new();
        match message.write_message(&mut payload) {
            Ok(()) => self.reliability_layer.send_packet(time, Priority::Highest, reliability, ordering, None, payload.into_boxed_slice()),
            Err(err) => error!("Failed writing message to buffer: {:?}", err),
        }
    }

    /// Returns true if this connection should be dropped.
    pub fn should_drop(&self, time: Instant, communicator: &mut Communicator<impl DatagramSocket>) -> bool {
        // TODO: Add more conditions and in some scenarios notify the user that the connection was closed.
        if (self.state == ConnectionState::UnverifiedSender || self.state == ConnectionState::HandlingConnectionRequest) &&
            time.saturating_duration_since(self.connection_time).as_millis() > communicator.config().incoming_connection_timeout_in_ms {
            debug!("Dropping connection from {} with guid {} because of connection timeout.", self.remote_addr, self.remote_guid);
            true
        } else if self.reliability_layer.is_dead_connection() {
            debug!("Dropping connection from {} with guid {} because of ack timeout.", self.remote_addr, self.remote_guid);
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ConnectionState {
    UnverifiedSender,
    HandlingConnectionRequest,
    Connected,
}