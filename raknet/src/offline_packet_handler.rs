use std::{
    collections::HashMap,
    convert::TryFrom,
    io::Cursor,
    net::SocketAddr
};

use log::debug;

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
    RakNetError,
    reader::RakNetMessageRead,
    socket::DatagramSocket,
    utils,
    writer::RakNetMessageWrite,
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
    pub fn process_offline_packet(&self, addr: SocketAddr, payload: &[u8], communicator: &mut Communicator<impl DatagramSocket>, connections: &mut HashMap<SocketAddr, Connection>) -> Result<bool, RakNetError>
    {
        // TODO: Check if remote peer is banned. If so, send MessageId::ConnectionBanned.

        if payload.len() <= 2 {
            debug!("Received too short packet. Length: {} bytes", payload.len());
            return Ok(true);
        }

        let mut reader = Cursor::new(payload);
        match MessageId::try_from(payload[0]) {
            Ok(MessageId::UnconnectedPing) => {
                if let Ok(ping) = UnconnectedPingMessage::read_message(&mut reader) {
                    self.handle_unconnected_ping_message(ping, addr, communicator)?;
                    return Ok(true);
                }
            },
            Ok(MessageId::UnconnectedPingOpenConnections) => {
                if let Ok(ping) = UnconnectedPingMessage::read_message(&mut reader) {
                    if Self::allow_incoming_connections(communicator.config(), connections) {
                        self.handle_unconnected_ping_message(ping, addr, communicator)?;
                    }
                    return Ok(true);
                }
            },
            Ok(MessageId::UnconnectedPong) => {
                if let Ok(pong) = UnconnectedPongMessage::read_message(&mut reader) {
                    self.handle_unconnected_pong_message(pong)?;
                    return Ok(true);
                }
            },
            Ok(MessageId::OpenConnectionRequest1) => {
                if let Ok(request1) = OpenConnectionRequest1Message::read_message(&mut reader) {
                    self.handle_open_connection_request1_message(request1, addr, communicator)?;
                    return Ok(true);
                }
            },
            Ok(MessageId::OpenConnectionRequest2) => {
                if let Ok(request2) = OpenConnectionRequest2Message::read_message(&mut reader) {
                    self.handle_open_connection_request2_message(request2, addr, communicator, connections)?;
                    return Ok(true);
                }
            },
            Ok(MessageId::OpenConnectionReply1) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::OpenConnectionReply2) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::OutOfBandInternal) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::ConnectionAttemptFailed) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::NoFreeIncomingConnections) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::ConnectionBanned) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::AlreadyConnected) => {
                // TODO: Implement
                return Ok(true);
            },
            Ok(MessageId::IpRecentlyConnected) => {
                // TODO: Implement
                return Ok(true);
            },
            _ => return Ok(false),
        }
        return Ok(false);
    }

    fn handle_unconnected_ping_message(&self, ping: UnconnectedPingMessage, addr: SocketAddr, communicator: &mut Communicator<impl DatagramSocket>) -> Result<(), RakNetError> {
        debug!("Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
        let pong = UnconnectedPongMessage { time: ping.time, guid: communicator.config().guid, data: self.ping_response.clone() };
        debug!("Sending Unconnected Pong");
        Self::send_message(&pong, addr, communicator)?;
        Ok(())
    }

    fn handle_unconnected_pong_message(&self, pong: UnconnectedPongMessage) -> Result<(), RakNetError> {
        debug!("Received Unconnected Pong: time={}, guid={}, data={:?}", pong.time, pong.guid, utils::to_hex(&pong.data, 40));
        // TODO: Forward event to user
        Ok(())
    }

    fn handle_open_connection_request1_message(&self, request1: OpenConnectionRequest1Message, addr: SocketAddr, communicator: &mut Communicator<impl DatagramSocket>) -> Result<(), RakNetError> {
        debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
        if request1.protocol_version != RAKNET_PROTOCOL_VERSION {
            let message = IncompatibleProtocolVersionMessage::new(RAKNET_PROTOCOL_VERSION, communicator.config().guid);
            debug!("Sending Incompatible Protocol Version");
            Self::send_message(&message, addr, communicator)?;
        } else {
            let requested_mtu = UDP_HEADER_SIZE + 1 + 16 + 1 + request1.padding_length;
            let mtu = if requested_mtu < MAXIMUM_MTU_SIZE { requested_mtu } else { MAXIMUM_MTU_SIZE };
            // TODO: Add support for security
            let response = OpenConnectionReply1Message::new(communicator.config().guid, None, mtu);
            debug!("Sending Open Connection Reply 1");
            Self::send_message(&response, addr, communicator)?
        }
        Ok(())
    }

    fn handle_open_connection_request2_message(&self, request2: OpenConnectionRequest2Message, addr: SocketAddr, communicator: &mut Communicator<impl DatagramSocket>, connections: &mut HashMap<SocketAddr, Connection>) -> Result<(), RakNetError> {
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
                let reply2 = OpenConnectionReply2Message::new(communicator.config().guid, addr, conn.mtu(), None);
                debug!("Sending Open Connection Reply2 (connection already exists)");
                Self::send_message(&reply2, addr, communicator)?;
                return Ok(());
            }
        }

        if guid_in_use || addr_in_use {
            // GUID or IP address already in use
            let message = ConnectErrorMessage::new(MessageId::AlreadyConnected, communicator.config().guid);
            debug!("Sending Already Connected");
            Self::send_message(&message, addr, communicator)?;
            return Ok(());
        }

        if !Self::allow_incoming_connections(communicator.config(), connections) {
            let message = ConnectErrorMessage::new(MessageId::NoFreeIncomingConnections, communicator.config().guid);
            debug!("Sending No Free Incoming Connections");
            Self::send_message(&message, addr, communicator)?;
            return Ok(());
        }

        // TODO: Check if this IP has connected the last 100 ms. If so, send MessageId::IpRecentlyConnected.
        // TODO: Check that the MTU is within our accepted range

        let conn = Connection::new(request2.guid, true, request2.mtu);
        connections.insert(addr, conn);

        // TODO: Add support for security and supply challenge answer.
        let reply2 = OpenConnectionReply2Message::new(communicator.config().guid, addr, request2.mtu, None);
        debug!("Sending Open Connection Reply 2");
        Self::send_message(&reply2, addr, communicator)?;

        Ok(())
    }

    fn allow_incoming_connections(config: &Config, connections: &HashMap<SocketAddr, Connection>) -> bool {
        let number_of_incoming_connections = connections.iter()
            .filter(|(_addr, conn)| conn.is_incoming() && conn.state == ConnectionState::Connected)
            .count();
        
        number_of_incoming_connections < config.max_incoming_connections
    }

    fn send_message(message: &dyn RakNetMessageWrite, dest: SocketAddr, communicator: &mut Communicator<impl DatagramSocket>) -> Result<(), RakNetError> {
        let mut payload = Vec::new();
        message.write_message(&mut payload)?;
        communicator.socket().send_datagram(&payload, dest)?;
        debug!("Sent {} bytes to {}: {}", payload.len(), dest, utils::to_hex(&payload, 40));
        Ok(())
    }   
}