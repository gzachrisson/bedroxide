use std::{
    collections::HashMap,
    convert::TryFrom,
    net::SocketAddr,
    time::Instant,
};

use log::{debug, error};

use crate::{
    communicator::Communicator,
    config::Config,
    connection::{Connection, ConnectionState},
    constants::{RAKNET_PROTOCOL_VERSION, UDP_HEADER_SIZE, MAXIMUM_MTU_SIZE},
    message_ids::MessageId,
    messages::{
        ConnectErrorMessage,
        IncompatibleProtocolVersionMessage,
        OpenConnectionRequest1Message,
        OpenConnectionRequest2Message,
        OpenConnectionReply1Message,
        OpenConnectionReply2Message,
        UnconnectedPingMessage,
        UnconnectedPongMessage,
    },
    reader::{OfflineMessageRead, DataReader},
    socket::DatagramSocket,
    utils,
    writer::OfflineMessageWrite,
};

pub struct  OfflinePacketHandler {   
    ping_response: Vec<u8>,
}

impl OfflinePacketHandler {
    pub fn new() -> OfflinePacketHandler {
        OfflinePacketHandler {
            ping_response: Vec::new(),
        }
    }

    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 bytes it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: Vec<u8>) 
    {
        let mut ping_response = ping_response;
        ping_response.truncate(399);
        self.ping_response = ping_response;
    }

    /// Process a possible offline packet.
    /// Returns true if the packet was handled.
    pub fn process_offline_packet(&self, time: Instant, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, connections: &mut HashMap<SocketAddr, Connection>) -> bool
    {
        // TODO: Check if remote peer is banned. If so, send MessageId::ConnectionBanned.

        if payload.len() > 2 {
            match MessageId::try_from(payload[0]) {
                Ok(MessageId::UnconnectedPing) => self.handle_unconnected_ping(addr, payload, communicator),
                Ok(MessageId::UnconnectedPingOpenConnections) => self.handle_unconnected_ping_open_connections(addr, payload, communicator, connections),
                Ok(MessageId::UnconnectedPong) => self.handle_unconnected_pong(addr, payload, communicator),
                Ok(MessageId::OpenConnectionRequest1) => self.handle_open_connection_request1(addr, payload, communicator),
                Ok(MessageId::OpenConnectionRequest2) => self.handle_open_connection_request2(time, addr, payload, communicator, connections),
                Ok(MessageId::OpenConnectionReply1) => {}, // TODO: Implement
                Ok(MessageId::OpenConnectionReply2) => {}, // TODO: Implement
                Ok(MessageId::OutOfBandInternal) => {}, // TODO: Implement
                Ok(MessageId::ConnectionAttemptFailed) => {}, // TODO: Implement
                Ok(MessageId::NoFreeIncomingConnections) => {}, // TODO: Implement
                Ok(MessageId::ConnectionBanned) => {}, // TODO: Implement
                Ok(MessageId::AlreadyConnected) => {}, // TODO: Implement
                Ok(MessageId::IpRecentlyConnected) => {}, // TODO: Implement
                _ => return false,
            }
        } else {
            debug!("Received too short packet. Length: {} bytes", payload.len());
        }

        true
    }

    fn handle_unconnected_ping(&self, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>) {
        let mut reader = DataReader::new(payload);
        match UnconnectedPingMessage::read_message(&mut reader) {
            Ok(ping) => {
                debug!("Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
                debug!("Sending Unconnected Pong");
                let pong = UnconnectedPongMessage::new(communicator.config().guid, ping.time, self.ping_response.clone());
                Self::send_message(&pong, addr, communicator);
            },
            Err(err) => error!("Could not read ping: {:?}", err),
        }
    }

    fn handle_unconnected_ping_open_connections(&self, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, connections: &mut HashMap<SocketAddr, Connection>) {
        if Self::allow_incoming_connections(communicator.config(), connections) {
            self.handle_unconnected_ping(addr, payload, communicator);
        }
    }

    fn handle_unconnected_pong(&self, _addr: SocketAddr, payload: &[u8], _communicator: &mut Communicator<impl DatagramSocket>) {
        let mut reader = DataReader::new(payload);
        match UnconnectedPongMessage::read_message(&mut reader) {
            Ok(pong) => {
                debug!("Received Unconnected Pong: time={}, guid={}, data={:?}", pong.time, pong.guid, utils::to_hex(&pong.data, 40));
                // TODO: Forward event to user
            },
            Err(err) => error!("Could not read pong: {:?}", err),
        }
    }

    fn handle_open_connection_request1(&self, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>) {
        let mut reader = DataReader::new(payload);
        match OpenConnectionRequest1Message::read_message(&mut reader) {
            Ok(request1) => {
                debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
                if request1.protocol_version != RAKNET_PROTOCOL_VERSION {
                    debug!("Sending Incompatible Protocol Version");
                    let message = IncompatibleProtocolVersionMessage::new(RAKNET_PROTOCOL_VERSION, communicator.config().guid);
                    Self::send_message(&message, addr, communicator);
                } else {
                    let requested_mtu = UDP_HEADER_SIZE + 1 + 16 + 1 + request1.padding_length;
                    let mtu = if requested_mtu < MAXIMUM_MTU_SIZE { requested_mtu } else { MAXIMUM_MTU_SIZE };
                    // TODO: Add support for security
                    debug!("Sending Open Connection Reply 1");
                    let response = OpenConnectionReply1Message::new(communicator.config().guid, None, mtu);
                    Self::send_message(&response, addr, communicator);
                }
            },
            Err(err) => error!("Could not read open connection request 1: {:?}", err),
        }
    }

    fn handle_open_connection_request2(&self, time: Instant, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, connections: &mut HashMap<SocketAddr, Connection>) {
        let mut reader = DataReader::new(payload);
        match OpenConnectionRequest2Message::read_message(&mut reader) {
            Ok(request2) => {
                debug!("Received Open Connection Request 2: mtu={} guid={} binding_address={:?}", request2.mtu, request2.guid, request2.binding_address);        
                
                // TODO: Check security if enabled

                let (guid_in_use, guid_in_use_by_same_addr) = connections.iter().find_map(|(remote_addr, conn)|
                    if conn.guid() == request2.guid {
                        Some((true, *remote_addr == addr))
                    } else {
                        None
                    }).unwrap_or((false, false));

                let (addr_in_use, addr_in_use_by_unverified_sender, conn) =
                    if let Some(conn) = connections.get_mut(&addr) {
                        (true, conn.state == ConnectionState::UnverifiedSender, Some(conn))
                    } else {
                        (false, false, None)
                    };
                
                if addr_in_use_by_unverified_sender && guid_in_use_by_same_addr {
                    if let Some(conn) = conn {
                        // Duplicate connection request due to packet loss
                        // Resend the reply
                        // TODO: Add support for security (resend challenge answer)
                        debug!("Sending Open Connection Reply2 (connection already exists)");
                        let reply2 = OpenConnectionReply2Message::new(communicator.config().guid, addr, conn.mtu(), None);
                        Self::send_message(&reply2, addr, communicator);
                        return;
                    }
                }

                if guid_in_use || addr_in_use {
                    // GUID or IP address already in use
                    debug!("Sending Already Connected");
                    let message = ConnectErrorMessage::new(MessageId::AlreadyConnected, communicator.config().guid);
                    Self::send_message(&message, addr, communicator);
                    return;
                }

                if !Self::allow_incoming_connections(communicator.config(), connections) {
                    debug!("Sending No Free Incoming Connections");
                    let message = ConnectErrorMessage::new(MessageId::NoFreeIncomingConnections, communicator.config().guid);
                    Self::send_message(&message, addr, communicator);
                    return;
                }

                // TODO: Check if this IP has connected the last 100 ms. If so, send MessageId::IpRecentlyConnected.
                // TODO: Check that the MTU is within our accepted range

                let conn = Connection::incoming(time, addr, request2.guid, request2.mtu);
                connections.insert(addr, conn);

                // TODO: Add support for security and supply challenge answer.
                debug!("Sending Open Connection Reply 2");
                let reply2 = OpenConnectionReply2Message::new(communicator.config().guid, addr, request2.mtu, None);
                Self::send_message(&reply2, addr, communicator);
            },
            Err(err) => error!("Failed reading open connection request 2: {:?}", err),
        }
    }

    fn allow_incoming_connections(config: &Config, connections: &HashMap<SocketAddr, Connection>) -> bool {
        // TODO: Revisit the logic below.
        // This logic is from the original RakNet C++ implementation. That we filter on ConnectionState::Connected
        // means that more incoming connections than `config.max_incoming_connections` are allowed as long as
        // they are in another state.
        let number_of_incoming_connections = connections.iter()
            .filter(|(_addr, conn)| conn.is_incoming() && conn.state == ConnectionState::Connected)
            .count();
        
        number_of_incoming_connections < config.max_incoming_connections
    }

    fn send_message(message: &dyn OfflineMessageWrite, dest: SocketAddr, communicator: &mut Communicator<impl DatagramSocket>) {
        let mut payload = Vec::new();
        match message.write_message(&mut payload) {
            Ok(()) => {
                match communicator.socket().send_datagram(&payload, dest) {
                    Ok(_) => debug!("Sent {} bytes to {}: {}", payload.len(), dest, utils::to_hex(&payload, 40)),
                    Err(err) => error!("Failed sending message: {:?}", err),
                }
            },
            Err(err) => error!("Failed writing message to buffer: {:?}", err),
        }
    }   
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::SocketAddr, time::Instant};
    use crossbeam_channel::{Receiver, unbounded};

    use crate::{        
        communicator::Communicator,
        config::Config,
        connection::{Connection, ConnectionState},
        message_ids::MessageId,
        messages::{ConnectErrorMessage, OpenConnectionRequest2Message, OpenConnectionReply2Message},
        offline_packet_handler::OfflinePacketHandler,
        reader::{OfflineMessageRead, DataReader},
        socket::FakeDatagramSocket,
        writer::OfflineMessageWrite,
    };

    const OWN_GUID: u64 = 0xFEDCBA9876453210;
    const REMOTE_GUID: u64 = 0xAABBCCDDEEFF0011;

    fn create_test_setup() -> (OfflinePacketHandler, Communicator<FakeDatagramSocket>, HashMap<SocketAddr, Connection>, Receiver<(Vec<u8>, SocketAddr)>, SocketAddr, SocketAddr) {
        let mut config = Config::default();
        config.guid = OWN_GUID;
        create_test_setup_with_config(config)
    }

    fn create_test_setup_with_config(config: Config) -> (OfflinePacketHandler, Communicator<FakeDatagramSocket>, HashMap<SocketAddr, Connection>, Receiver<(Vec<u8>, SocketAddr)>, SocketAddr, SocketAddr) {
        let socket = FakeDatagramSocket::new();
        let datagram_receiver = socket.get_datagram_receiver();
        let (event_sender, _event_receiver) = unbounded();
        let communicator = Communicator::new(socket, config, event_sender);
        let connections = HashMap::<SocketAddr, Connection>::new();
        let remote_addr = "192.168.1.1:19132".parse::<SocketAddr>().expect("Could not create address");
        let own_addr = "127.0.0.1:19132".parse::<SocketAddr>().expect("Could not create address");
        (OfflinePacketHandler::new(), communicator, connections, datagram_receiver, remote_addr, own_addr)
    }    

    fn receive_datagram<M: OfflineMessageRead>(datagram_receiver: &mut Receiver<(Vec<u8>, SocketAddr)>) -> (M, SocketAddr) {
        let (payload, addr) = datagram_receiver.try_recv().expect("Datagram not received");
        let mut reader = DataReader::new(&payload);
        let message = M::read_message(&mut reader).expect("Could not parse message");
        (message, addr)
    }

    #[test]
    fn open_connection_request_2_guid_and_addr_in_use_by_remote() {
        // Arrange
        let (handler, mut communicator, mut connections, mut datagram_receiver, remote_addr, own_addr) = create_test_setup();
        let mut payload = Vec::new();
        let message = OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address: own_addr,
            mtu: 1024,
            guid: REMOTE_GUID,
        };
        message.write_message(&mut payload).expect("Could not write message");
        connections.insert(remote_addr, Connection::incoming(Instant::now(), remote_addr, REMOTE_GUID, 1024));

        // Act
        let handled = handler.process_offline_packet(Instant::now(), remote_addr, &payload, &mut communicator, &mut connections);

        // Assert
        let (message, addr) = receive_datagram::<OpenConnectionReply2Message>(&mut datagram_receiver);
        assert_eq!(true, handled);
        assert_eq!(remote_addr, addr);
        assert_eq!(OWN_GUID, message.guid);
        assert_eq!(remote_addr, message.client_address);
        assert_eq!(1024, message.mtu);
        assert_eq!(None, message.challenge_answer);
    }

    #[test]
    fn open_connection_request_2_guid_in_use_by_other() {
        // Arrange
        let (handler, mut communicator, mut connections, mut datagram_receiver, remote_addr, own_addr) = create_test_setup();
        let mut payload = Vec::new();
        let message = OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address: own_addr,
            mtu: 1024,
            guid: REMOTE_GUID,
        };
        message.write_message(&mut payload).expect("Could not write message");
        let other_addr = "192.168.1.99:19132".parse::<SocketAddr>().expect("Could not create address");
        connections.insert(other_addr, Connection::incoming(Instant::now(), remote_addr, REMOTE_GUID, 1024));

        // Act
        let handled = handler.process_offline_packet(Instant::now(), remote_addr, &payload, &mut communicator, &mut connections);

        // Assert
        let (message, addr) = receive_datagram::<ConnectErrorMessage>(&mut datagram_receiver);
        assert_eq!(true, handled);        
        assert_eq!(remote_addr, addr);
        assert_eq!(MessageId::AlreadyConnected, message.message_id);
        assert_eq!(OWN_GUID, message.guid);
    }

    #[test]
    fn open_connection_request_2_addr_in_use_with_other_guid() {
        // Arrange
        let (handler, mut communicator, mut connections, mut datagram_receiver, remote_addr, own_addr) = create_test_setup();
        let mut payload = Vec::new();
        let message = OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address: own_addr,
            mtu: 1024,
            guid: REMOTE_GUID,
        };
        message.write_message(&mut payload).expect("Could not write message");
        let other_guid: u64 = 0x1111111111111111;
        connections.insert(remote_addr, Connection::incoming(Instant::now(), remote_addr, other_guid, 1024));

        // Act
        let handled = handler.process_offline_packet(Instant::now(), remote_addr, &payload, &mut communicator, &mut connections);

        // Assert
        let (message, addr) = receive_datagram::<ConnectErrorMessage>(&mut datagram_receiver);
        assert_eq!(true, handled);        
        assert_eq!(remote_addr, addr);
        assert_eq!(MessageId::AlreadyConnected, message.message_id);
        assert_eq!(OWN_GUID, message.guid);
    }

    #[test]
    fn open_connection_request_2_max_incoming_connections_exceeded() {
        // Arrange
        let mut config = Config::default();
        config.guid = OWN_GUID;
        config.max_incoming_connections = 1;
        let (handler, mut communicator, mut connections, mut datagram_receiver, remote_addr, own_addr) = create_test_setup_with_config(config);
        let mut payload = Vec::new();
        let message = OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address: own_addr,
            mtu: 1024,
            guid: REMOTE_GUID,
        };
        message.write_message(&mut payload).expect("Could not write message");
        let other_guid: u64 = 0x1111111111111111;
        let other_addr = "192.168.1.99:19132".parse::<SocketAddr>().expect("Could not create address");
        let mut connection = Connection::incoming(Instant::now(), remote_addr, other_guid, 1024);
        connection.state = ConnectionState::Connected;
        connections.insert(other_addr, connection);

        // Act
        let handled = handler.process_offline_packet(Instant::now(), remote_addr, &payload, &mut communicator, &mut connections);

        // Assert
        let (message, addr) = receive_datagram::<ConnectErrorMessage>(&mut datagram_receiver);
        assert_eq!(true, handled);        
        assert_eq!(remote_addr, addr);
        assert_eq!(MessageId::NoFreeIncomingConnections, message.message_id);
        assert_eq!(OWN_GUID, message.guid);
    }       
}