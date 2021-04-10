use std::{
    io::Cursor,
    net::{SocketAddr, UdpSocket},
};

use log::{error, debug};
use rand;

use super::{
    RakNetError,
    messages::{OpenConnectionRequest1Message, UnconnectedPingMessage, UnconnectedPongMessage},
    reader::{RakNetMessageRead},
    writer::{RakNetMessageWrite},
    utils,
};

pub struct ConnectionManager {
    socket: UdpSocket,
    ping_response: String,
    guid: u64,
    receive_buffer: Vec<u8>,
}

impl ConnectionManager {
    pub fn new(socket: UdpSocket) -> Self {
        let ping_response = String::new();
        let guid = rand::random();
        let receive_buffer = vec![0u8; 2048];
        ConnectionManager {
            socket,
            ping_response,
            guid,
            receive_buffer,
        }
    }

    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 chars it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: String) 
    {
        let mut ping_response = ping_response;
        ping_response.truncate(399);
        self.ping_response = ping_response;
    }

    /// Sends and receives packages/events and updates connections.
    pub fn process(&mut self) {
        // Process all incoming packets
        loop
        {
            match self.socket.recv_from(self.receive_buffer.as_mut())
            {
                Ok((received_length, addr)) => {
                    debug!("Received {} bytes from {}: {}", received_length, addr, utils::to_hex(&self.receive_buffer[..received_length.min(40)]));
                    let payload = self.receive_buffer[..received_length].as_ref();
                    match self.process_offline_packet(addr, payload)
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

    fn process_offline_packet(&self, addr: SocketAddr, payload: &[u8]) -> Result<bool, RakNetError>
    {        
        let mut reader = Cursor::new(payload);
        if let Ok(ping) = UnconnectedPingMessage::read_message(&mut reader) {
            self.handle_unconnected_ping_message(ping, addr)?;
            return Ok(true);
        }
        
        reader.set_position(0);
        if let Ok(pong) = UnconnectedPongMessage::read_message(&mut reader) {
            self.handle_unconnected_pong_message(pong, addr)?;
            return Ok(true);
        }

        reader.set_position(0);
        if let Ok(request1) = OpenConnectionRequest1Message::read_message(&mut reader) {
            self.handle_open_connection_request1_message(request1, addr)?;
            return Ok(true);
        }
         
        debug!("Unhandled message ID: {}", payload[0]);        
        Ok(false)
    }

    fn handle_unconnected_ping_message(&self, ping: UnconnectedPingMessage, addr: SocketAddr) -> Result<(), RakNetError> {
        debug!("Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
        let pong = UnconnectedPongMessage { time: ping.time, guid: self.guid, data: self.ping_response.clone() };
        self.send_message(&pong, addr)?;
        debug!("Sent Unconnected Pong");
        Ok(())
    }

    fn handle_unconnected_pong_message(&self, pong: UnconnectedPongMessage, _addr: SocketAddr) -> Result<(), RakNetError> {
        debug!("Received Unconnected Pong: time={}, guid={}, data={}", pong.time, pong.guid, pong.data);
        // TODO: Forward event to user
        Ok(())
    }

    fn handle_open_connection_request1_message(&self, request1: OpenConnectionRequest1Message, _addr: SocketAddr) -> Result<(), RakNetError> {
        debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
        // TODO: Send response
        Ok(())
    }

    fn send_message(&self, message: &dyn RakNetMessageWrite, dest: SocketAddr) -> Result<(), RakNetError> {
        let mut buf = Vec::new();
        message.write_message(&mut buf)?;
        self.socket.send_to(&buf, dest)?;
        debug!("Sent {} bytes to {}: {}", buf.len(), dest, utils::to_hex(&buf[..buf.len().min(40)]));
        Ok(())
    }   
}