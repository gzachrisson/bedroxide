use std::{net::SocketAddr, collections::HashMap, time::Instant};
use crossbeam_channel::{unbounded, Receiver};
use log::{error, debug};

use crate::{
    communicator::Communicator,
    config::Config,
    connection::Connection,
    constants::MAXIMUM_MTU_SIZE,
    offline_packet_handler::OfflinePacketHandler,
    PeerEvent,
    socket::DatagramSocket,
    utils,
};

pub struct ConnectionManager<T: DatagramSocket> {
    communicator: Communicator<T>,
    connections: HashMap<SocketAddr, Connection>,
    event_receiver: Receiver<PeerEvent>,
    offline_packet_handler: OfflinePacketHandler,
    receive_buffer: Vec<u8>,
}

impl<T: DatagramSocket> ConnectionManager<T> {
    pub fn new(socket: T, config: Config) -> Self {
        let receive_buffer = vec![0u8; MAXIMUM_MTU_SIZE.into()];
        let (event_sender, event_receiver) = unbounded();
        ConnectionManager {
            communicator: Communicator::new(socket, config, event_sender),
            connections: HashMap::new(),
            event_receiver,
            offline_packet_handler: OfflinePacketHandler::new(),
            receive_buffer,
        }
    }

    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 bytes it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: Vec<u8>) 
    {
        self.offline_packet_handler.set_offline_ping_response(ping_response);
    }

    /// Sends and receives packages/events and updates connections.
    pub fn process(&mut self, time: Instant) {
        let communicator = &mut self.communicator;

        // Process all incoming packets
        loop
        {
            match communicator.socket().receive_datagram(self.receive_buffer.as_mut())
            {
                Ok((payload, addr)) => {
                    debug!("Received {} bytes from {}: {}", payload.len(), addr, utils::to_hex(&payload, 40));
                    if !self.offline_packet_handler.process_offline_packet(time, addr, payload, communicator, &mut self.connections) {
                        if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.process_incoming_datagram(payload, time, communicator);
                        }
                    }
                },
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock {
                        error!("Error receiving from socket: {:?}", err);                    
                    }
                    break;
                }
            }
        }

        // Update all connections
        for conn in self.connections.values_mut() {
            conn.update(time, communicator);
        }

        // Check if any connection should be dropped
        self.connections.retain(|_, conn| !conn.should_drop(time, communicator));
    }

    /// Gets an event receiver that can be used for receiving
    /// incoming packets and connection events.
    pub fn event_receiver(&self) -> Receiver<PeerEvent> {
        self.event_receiver.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Instant};   
    use crossbeam_channel::{Sender, Receiver};
    use crate::{
        config::Config,
        connection_manager::ConnectionManager,
        constants::{RAKNET_PROTOCOL_VERSION, UDP_HEADER_SIZE},
        message_ids::MessageId,
        messages::{
            IncompatibleProtocolVersionMessage,
            OpenConnectionReply1Message,
            OpenConnectionReply2Message,
            OpenConnectionRequest1Message,
            OpenConnectionRequest2Message,
            UnconnectedPingMessage,
            UnconnectedPongMessage,
        },
        reader::{MessageRead, DataReader},
        socket::FakeDatagramSocket,
        writer::MessageWrite,
    };

    const OWN_GUID: u64 = 0xFEDCBA9876453210;

    fn create_connection_manager() -> (ConnectionManager<FakeDatagramSocket>, Sender<(Vec<u8>, SocketAddr)>, Receiver<(Vec<u8>, SocketAddr)>, SocketAddr) {
        let local_addr = "127.0.0.2:19132".parse::<SocketAddr>().expect("Could not create address");
        let fake_socket = FakeDatagramSocket::new(local_addr);
        let datagram_sender = fake_socket.get_datagram_sender();
        let datagram_receiver = fake_socket.get_datagram_receiver();
        let remote_addr = "127.0.0.1:19132".parse::<SocketAddr>().expect("Could not create address");
        let mut config = Config::default();
        config.guid = OWN_GUID;
        (ConnectionManager::new(fake_socket, config), datagram_sender, datagram_receiver, remote_addr)
    }

    fn send_datagram<M: MessageWrite>(message: M, datagram_sender: &mut Sender<(Vec<u8>, SocketAddr)>, remote_addr: SocketAddr) {
        let mut buf = Vec::new();
        message.write_message(&mut buf).expect("Could not create message");
        datagram_sender.send((buf, remote_addr)).expect("Could not send datagram");
    }

    fn receive_datagram<M: MessageRead>(datagram_receiver: &mut Receiver<(Vec<u8>, SocketAddr)>) -> (M, SocketAddr) {
        let (payload, addr) = datagram_receiver.try_recv().expect("Datagram not received");
        let mut reader = DataReader::new(&payload);
        let message = M::read_message(&mut reader).expect("Could not parse message");
        (message, addr)
    }

    #[test]
    fn ping_responds_with_pong() {
        // Arrange
        let (mut connection_manager, mut datagram_sender, mut datagram_receiver, remote_addr) = create_connection_manager();
        let ping = UnconnectedPingMessage {
            message_id: MessageId::UnconnectedPing,
            time: 0x0123456789ABCDEF,
            client_guid: 0x1122334455667788,
        };
        connection_manager.set_offline_ping_response(vec![0x00, 0x02, 0x41, 0x42]);
        send_datagram(ping, &mut datagram_sender, remote_addr);
        
        // Act
        connection_manager.process(Instant::now());

        // Assert
        let (pong, addr) = receive_datagram::<UnconnectedPongMessage>(&mut datagram_receiver);
        assert_eq!(remote_addr, addr);
        assert_eq!(0x0123456789ABCDEF, pong.time);
        assert_eq!(OWN_GUID, pong.guid);
        assert_eq!(vec![0x00, 0x02, 0x41, 0x42], pong.data);
    }

    #[test]
    fn open_connection_request_1_incompatible_protocol_version() {
        // Arrange
        let (mut connection_manager, mut datagram_sender, mut datagram_receiver, remote_addr) = create_connection_manager();
        let req1 = OpenConnectionRequest1Message {
            protocol_version: RAKNET_PROTOCOL_VERSION + 1, // INVALID protocol version
            padding_length: 8,
        };
        send_datagram(req1, &mut datagram_sender, remote_addr);

        // Act
        connection_manager.process(Instant::now());

        // Assert
        let (message, addr) = receive_datagram::<IncompatibleProtocolVersionMessage>(&mut datagram_receiver);
        assert_eq!(remote_addr, addr);      
        assert_eq!(RAKNET_PROTOCOL_VERSION, message.protocol_version);
        assert_eq!(OWN_GUID, message.guid);
    }

    #[test]
    fn open_connection_request_1_responds_with_reply_1() {
        // Arrange
        let (mut connection_manager, mut datagram_sender, mut datagram_receiver, remote_addr) = create_connection_manager();
        let req1 = OpenConnectionRequest1Message {
            protocol_version: RAKNET_PROTOCOL_VERSION,
            padding_length: 400,
        };
        send_datagram(req1, &mut datagram_sender, remote_addr);

        // Act
        connection_manager.process(Instant::now());

        // Assert
        let (message, addr) = receive_datagram::<OpenConnectionReply1Message>(&mut datagram_receiver);
        assert_eq!(remote_addr, addr);
        assert_eq!(OWN_GUID, message.guid);
        assert_eq!(None, message.cookie_and_public_key);
        assert_eq!(UDP_HEADER_SIZE + 1 + 16 + 1 + 400, message.mtu);
    }

    #[test]
    fn open_connection_request_2_responds_with_reply_2() {
        // Arrange
        let (mut connection_manager, mut datagram_sender, mut datagram_receiver, remote_addr) = create_connection_manager();
        let req2 = OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address: SocketAddr::from(([192, 168, 1, 248], 0x1234)),
            mtu: 446,
            guid: 0x12345678,
        };
        send_datagram(req2, &mut datagram_sender, remote_addr);

        // Act
        connection_manager.process(Instant::now());

        // Assert
        let (message, addr) = receive_datagram::<OpenConnectionReply2Message>(&mut datagram_receiver);
        assert_eq!(remote_addr, addr);
        assert_eq!(OWN_GUID, message.guid);
        assert_eq!(remote_addr, message.client_address);
        assert_eq!(446, message.mtu);
        assert_eq!(None, message.challenge_answer);
    }     
}