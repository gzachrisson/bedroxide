use std::{
    io::Cursor,
    net::{SocketAddr},
};

use log::{error, debug};

use crate::{
    config::Config,
    messages::{OpenConnectionRequest1Message, UnconnectedPingMessage, UnconnectedPongMessage},
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
    /// If the response is longer than 399 chars it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: String) 
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
                    debug!("Received {} bytes from {}: {}", payload.len(), addr, utils::to_hex(&payload[..payload.len().min(40)]));
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
            Self::handle_open_connection_request1_message(request1)?;
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
        debug!("Received Unconnected Pong: time={}, guid={}, data={}", pong.time, pong.guid, pong.data);
        // TODO: Forward event to user
        Ok(())
    }

    fn handle_open_connection_request1_message(request1: OpenConnectionRequest1Message) -> Result<(), RakNetError> {
        debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
        // TODO: Send response
        Ok(())
    }

    fn send_message(message: &dyn RakNetMessageWrite, dest: SocketAddr, socket: &mut T) -> Result<(), RakNetError> {
        let mut payload = Vec::new();
        message.write_message(&mut payload)?;
        socket.send_datagram(&payload, dest)?;
        debug!("Sent {} bytes to {}: {}", payload.len(), dest, utils::to_hex(&payload[..payload.len().min(40)]));
        Ok(())
    }   
}

#[cfg(test)]
mod tests {
 
    #[test]
    fn ping_responds_with_pong() {
        
    }
}