use std::{
    io::Cursor,
    net::{SocketAddr},
};

use log::{error, debug};

use crate::{
    config::Config,
    constants::RAKNET_PROTOCOL_VERSION,
    messages::{
        OpenConnectionRequest1Message,
        UnconnectedPingMessage,
        UnconnectedPongMessage,
        IncompatibleProtocolVersionMessage
    },
    RakNetError,
    reader::{RakNetMessageRead},
    socket::DatagramSocket,
    utils,
    writer::{RakNetMessageWrite},
};

pub struct ConnectionManager<T: DatagramSocket> {
    socket: T,
    config: Config,
    receive_buffer: Vec<u8>,
}

impl<T: DatagramSocket> ConnectionManager<T> {
    pub fn new(socket: T, config: Config) -> Self {
        let receive_buffer = vec![0u8; 2048];
        ConnectionManager {
            socket,
            config,
            receive_buffer,
        }
    }

    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 bytes it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: Vec<u8>) 
    {
        let mut ping_response = ping_response;
        ping_response.truncate(399);
        self.config.ping_response = ping_response;
    }

    /// Sends and receives packages/events and updates connections.
    pub fn process(&mut self) {
        // Process all incoming packets
        loop
        {
            match self.socket.receive_datagram(self.receive_buffer.as_mut())
            {
                Ok((payload, addr)) => {
                    debug!("Received {} bytes from {}: {}", payload.len(), addr, utils::to_hex(&payload, 40));
                    match Self::process_offline_packet(addr, payload, &mut self.socket, &self.config)
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

    fn process_offline_packet(addr: SocketAddr, payload: &[u8], socket: &mut T, config: &Config) -> Result<bool, RakNetError>
    {        
        let mut reader = Cursor::new(payload);
        if let Ok(ping) = UnconnectedPingMessage::read_message(&mut reader) {
            Self::handle_unconnected_ping_message(ping, addr, socket, config)?;
            return Ok(true);
        }
        
        reader.set_position(0);
        if let Ok(pong) = UnconnectedPongMessage::read_message(&mut reader) {
            Self::handle_unconnected_pong_message(pong)?;
            return Ok(true);
        }

        reader.set_position(0);
        if let Ok(request1) = OpenConnectionRequest1Message::read_message(&mut reader) {
            Self::handle_open_connection_request1_message(request1, addr, socket, config)?;
            return Ok(true);
        }
         
        debug!("Unhandled message ID: {}", payload[0]);        
        Ok(false)
    }

    fn handle_unconnected_ping_message(ping: UnconnectedPingMessage, addr: SocketAddr, socket: &mut T, config: &Config) -> Result<(), RakNetError> {
        debug!("Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
        let pong = UnconnectedPongMessage { time: ping.time, guid: config.guid, data: config.ping_response.clone() };
        Self::send_message(&pong, addr, socket)?;
        debug!("Sent Unconnected Pong");
        Ok(())
    }

    fn handle_unconnected_pong_message(pong: UnconnectedPongMessage) -> Result<(), RakNetError> {
        debug!("Received Unconnected Pong: time={}, guid={}, data={:?}", pong.time, pong.guid, utils::to_hex(&pong.data, 40));
        // TODO: Forward event to user
        Ok(())
    }

    fn handle_open_connection_request1_message(request1: OpenConnectionRequest1Message, addr: SocketAddr, socket: &mut T, config: &Config) -> Result<(), RakNetError> {
        debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
        if request1.protocol_version != RAKNET_PROTOCOL_VERSION {
            let response = IncompatibleProtocolVersionMessage {
                protocol_version: RAKNET_PROTOCOL_VERSION,
                guid: config.guid,
            };
            Self::send_message(&response, addr, socket)?;
        } else {
            // TODO: Send response
        }
        Ok(())
    }

    fn send_message(message: &dyn RakNetMessageWrite, dest: SocketAddr, socket: &mut T) -> Result<(), RakNetError> {
        let mut payload = Vec::new();
        message.write_message(&mut payload)?;
        socket.send_datagram(&payload, dest)?;
        debug!("Sent {} bytes to {}: {}", payload.len(), dest, utils::to_hex(&payload, 40));
        Ok(())
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
        constants::RAKNET_PROTOCOL_VERSION,
        messages::{UnconnectedPingMessage, UnconnectedPongMessage, OpenConnectionRequest1Message, IncompatibleProtocolVersionMessage},
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
}