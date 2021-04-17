use log::{error, debug};

use crate::{
    communicator::Communicator,
    config::Config,
    constants::MAXIMUM_MTU_SIZE,
    offline_packet_handler::OfflinePacketHandler,
    socket::DatagramSocket,
    utils,
};

pub struct ConnectionManager<T: DatagramSocket> {
    communicator: Communicator<T>,
    offline_packet_handler: OfflinePacketHandler,
    receive_buffer: Vec<u8>,
}

impl<T: DatagramSocket> ConnectionManager<T> {
    pub fn new(socket: T, config: Config) -> Self {
        let receive_buffer = vec![0u8; MAXIMUM_MTU_SIZE.into()];
        ConnectionManager {
            communicator: Communicator::new(socket, config),
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
    pub fn process(&mut self) {
        // Process all incoming packets
        loop
        {
            match self.communicator.socket().receive_datagram(self.receive_buffer.as_mut())
            {
                Ok((payload, addr)) => {
                    debug!("Received {} bytes from {}: {}", payload.len(), addr, utils::to_hex(&payload, 40));
                    match self.offline_packet_handler.process_offline_packet(addr, payload, &mut self.communicator)
                    {
                        Ok(true) => continue,
                        Ok(false) => { 
                            // TODO: Process online packet
                        }
                        Err(err) => error!("Error when processing received packet: {:?}", err),
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
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Cursor,
        net::{SocketAddr},
    };   
    use crossbeam_channel::{Sender, Receiver};
    use crate::{
        config::Config,
        connection_manager::ConnectionManager,
        constants::{RAKNET_PROTOCOL_VERSION, UDP_HEADER_SIZE},
        messages::{
            UnconnectedPingMessage,
            UnconnectedPongMessage,
            OpenConnectionRequest1Message,
            OpenConnectionReply1Message,
            IncompatibleProtocolVersionMessage
        },
        reader::RakNetMessageRead,
        socket::FakeDatagramSocket,
        writer::RakNetMessageWrite,
    };
    const OWN_GUID: u64 = 0xFEDCBA9876453210;

    fn create_connection_manager() -> (ConnectionManager<FakeDatagramSocket>, Sender<(Vec<u8>, SocketAddr)>, Receiver<(Vec<u8>, SocketAddr)>, SocketAddr) {
        let fake_socket = FakeDatagramSocket::new();
        let datagram_sender = fake_socket.get_datagram_sender();
        let datagram_receiver = fake_socket.get_datagram_receiver();
        let remote_addr = "127.0.0.1:19132".parse::<SocketAddr>().expect("Could not create address");
        let mut config = Config::default();
        config.guid = OWN_GUID;
        (ConnectionManager::new(fake_socket, config), datagram_sender, datagram_receiver, remote_addr)
    }

    fn send_datagram<M: RakNetMessageWrite>(message: M, datagram_sender: &mut Sender<(Vec<u8>, SocketAddr)>, remote_addr: SocketAddr) {
        let mut buf = Vec::new();
        message.write_message(&mut buf).expect("Could not create message");
        datagram_sender.send((buf, remote_addr)).expect("Could not send datagram");
    }

    fn receive_datagram<M: RakNetMessageRead>(datagram_receiver: &mut Receiver<(Vec<u8>, SocketAddr)>) -> (M, SocketAddr) {
        let (payload, addr) = datagram_receiver.try_recv().expect("Datagram not received");
        let mut reader = Cursor::new(payload);
        let message = M::read_message(&mut reader).expect("Could not parse message");
        (message, addr)
    }

    #[test]
    fn ping_responds_with_pong() {
        // Arrange
        let (mut connection_manager, mut datagram_sender, mut datagram_receiver, remote_addr) = create_connection_manager();
        let ping = UnconnectedPingMessage {
            time: 0x0123456789ABCDEF,
            client_guid: 0x1122334455667788,
        };
        connection_manager.set_offline_ping_response(vec![0x00, 0x02, 0x41, 0x42]);
        send_datagram(ping, &mut datagram_sender, remote_addr);
        
        // Act
        connection_manager.process();

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
        connection_manager.process();

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
        connection_manager.process();

        // Assert
        let (message, addr) = receive_datagram::<OpenConnectionReply1Message>(&mut datagram_receiver);
        assert_eq!(remote_addr, addr);
        assert_eq!(OWN_GUID, message.guid);
        assert_eq!(None, message.cookie_and_public_key);
        assert_eq!(UDP_HEADER_SIZE + 1 + 16 + 1 + 400, message.mtu);
    } 
}