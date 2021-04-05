use std::{
    convert::{TryFrom, TryInto},
    io::{Write, Read, Cursor},
    net::{SocketAddr},
    thread::{sleep},
    time::Duration,
};

use std::net::{UdpSocket, ToSocketAddrs};
use log::{info, error, debug};
use rand;

#[derive(Debug)]
pub enum RakNetError {
    IoError(std::io::Error),
    TooFewBytesWritten(usize),
    TooFewBytesRead(usize),
    StringParseError(std::string::FromUtf8Error)
}

pub struct RakNetPeer
{
    socket: UdpSocket,
    unconnected_ping_response: String,
    guid: u64,
    receive_buffer: Vec<u8>,
}

impl RakNetPeer {
    /// Creates a RakNetPeer and binds it to a UDP socket on the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, RakNetError> {
        info!("Binding socket");
        let socket = UdpSocket::bind(addr)?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;

        info!("Listening on {}", socket.local_addr()?);
        
        Ok(RakNetPeer {
            socket,
            unconnected_ping_response: String::new(),
            guid: rand::random(),
            receive_buffer: vec![0u8; 2048],
        })
    }

    /// Sends and receives packages/events and updates connections.
    /// 
    /// Returns true if the caller should continue to call the
    /// method and false if the peer is shutting down and
    /// no more updates should be done.
    /// 
    /// Use `process` to manually decide when to process network
    /// events. For an automatic processing loop use `start_processing`
    /// or `start_processing_with_duration` instead.
    pub fn process(&mut self) -> bool {
        // Process all incoming packets
        loop
        {
            match self.socket.recv_from(self.receive_buffer.as_mut())
            {
                Ok((n, addr)) => {
                    if n == 0 {
                        error!("Received 0 byte message from {}", addr);
                        return true;
                    }
                    debug!("Received {} bytes from {}: {}", n, addr, Self::to_hex(&self.receive_buffer, n.min(40)));

                    match self.process_received_packet(addr)
                    {
                        Ok(_) => {}
                        Err(err) => error!("Error when processing received packet: {:?}", err),
                    }
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::WouldBlock
                    {
                        error!("Error receiving from socket: {:?}", err);                    
                    }
                    break;
                }
            }
        }

        true
    }

    /// Starts a loop that processes incoming and outgoing
    /// packets with a default sleep time of 1 ms between processing.
    /// 
    /// This method blocks and should be called from a spawned thread.
    pub fn start_processing(&mut self) {       
        self.start_processing_with_duration(Duration::from_millis(1));
    }

    /// Starts a loop that processes incoming and outgoing
    /// packets with the specified sleep time between the processing rounds.
    /// 
    /// This method blocks and should be called from a spawned thread.
    pub fn start_processing_with_duration(&mut self, sleep_time: Duration) {       
        loop {
            if !self.process() {
                break;
            }
            sleep(sleep_time);
        }
    }    
    
    /// Sets the message returned in an unconnected ping response.
    pub fn set_unconnected_ping_response(&mut self, unconnected_ping_response: &str)
    {
        self.unconnected_ping_response = unconnected_ping_response.to_string();
    }

    fn process_received_packet(&mut self, addr: SocketAddr) -> Result<(), RakNetError>
    {
        let mut reader = Cursor::new(&self.receive_buffer);
        let message_id = reader.read_byte()?;

        match message_id {
            // Unconnected Ping        
            0x01 => {
                let ping = UnconnectedPingMessage::read_message(&mut reader)?;
                debug!("  Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
    
                // Send Unconnected Pong
                let pong = UnconnectedPongMessage { time: ping.time, guid: self.guid, data: self.unconnected_ping_response.clone() };
                self.send_message(&pong, addr)?;
                debug!("  Sent Unconnected Pong");
            },
            
            // Open Connection Request 1
            0x05 => {
                let request = OpenConnectionRequest1Message::read_message(&mut reader)?;
                debug!("  Received Open Connection Request 1: protocol_version={}, padding_length={}", request.protocol_version, request.padding_length);
            },
            
            _ => {
                debug!("  Received unknown message ID: {}", message_id);
            }
        }
        Ok(())
    }

    fn send_message(&self, message: &dyn RakNetMessageWrite, dest: SocketAddr) -> Result<(), RakNetError> {
        let mut send_buf = Vec::with_capacity(1024); // TODO: Allocate once
        send_buf.write_byte(message.message_id())?;
        message.write_message(&mut send_buf)?;
        self.socket.send_to(&send_buf, dest)?;
        debug!("Sent {} bytes to {}: {}", send_buf.len(), dest, Self::to_hex(&send_buf, send_buf.len().min(40)));
        Ok(())
    }
    
    fn to_hex(buf: &Vec<u8>, n: usize) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for &byte in buf.iter().take(n) {
            write!(&mut s, "{:02X} ", byte).expect("Unable to write");
        }
        return s;
    }    
}

trait RakNetMessageRead: Sized {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError>;
}

trait RakNetMessageWrite {
    fn message_id(&self) -> u8;
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError>;
}

struct UnconnectedPingMessage {
    pub time: u64,
    pub client_guid: u64
}

impl RakNetMessageRead for UnconnectedPingMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        let time = reader.read_unsigned_long_be()?;
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let client_guid = reader.read_unsigned_long_be()?;
        Ok(UnconnectedPingMessage { time, client_guid })
    }
}

impl RakNetMessageWrite for UnconnectedPingMessage {
    fn message_id(&self) -> u8 { 0x01 }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_unsigned_long_be(self.time)?;
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_unsigned_long_be(self.client_guid)?;
        Ok(())
    }
}

struct UnconnectedPongMessage {
    pub time: u64,
    pub guid: u64,
    pub data: String
}

impl RakNetMessageRead for UnconnectedPongMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        let time = reader.read_unsigned_long_be()?;
        let guid = reader.read_unsigned_long_be()?;
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let data = reader.read_fixed_string()?;
        Ok(UnconnectedPongMessage { time, guid, data })
    }
}

impl RakNetMessageWrite for UnconnectedPongMessage {
    fn message_id(&self) -> u8 { 0x1c }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_unsigned_long_be(self.time)?;
        writer.write_unsigned_long_be(self.guid)?;
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_fixed_string(&self.data)?;
        Ok(())      
    }
}

struct OpenConnectionRequest1Message {
    pub protocol_version: u8,
    pub padding_length: u16
}

impl RakNetMessageRead for OpenConnectionRequest1Message {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let protocol_version = reader.read_byte()?;
        let padding_length = reader.read_zero_padding()?;
        Ok(OpenConnectionRequest1Message { protocol_version, padding_length })
    }
}

impl RakNetMessageWrite for OpenConnectionRequest1Message {
    fn message_id(&self) -> u8 { 0x05 }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_byte(self.protocol_version)?;
        writer.write_zero_padding(self.padding_length)?;
        Ok(())      
    }
}

impl From<std::io::Error> for RakNetError {
    fn from(error: std::io::Error) -> Self {
        RakNetError::IoError(error)
    }
}

impl From<std::string::FromUtf8Error> for RakNetError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        RakNetError::StringParseError(error)
    }
}

trait RakNetWrite {
    fn write_byte(&mut self, b: u8) -> Result<usize, RakNetError>;
    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, RakNetError>;
    fn write_unsigned_short_be(&mut self, us: u16) -> Result<usize, RakNetError>;
    fn write_unsigned_long_be(&mut self, ul: u64) -> Result<usize, RakNetError>;
    fn write_fixed_string(&mut self, s: &str) -> Result<usize, RakNetError>;
    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize, RakNetError>;
}

impl<T> RakNetWrite for T where T: Write {
    fn write_byte(&mut self, b: u8) -> Result<usize, RakNetError> {
        let n = self.write(&[b])?;
        if n != 1 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, RakNetError> {
        let n = self.write(b)?;
        if n != b.len() {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_unsigned_short_be(&mut self, us: u16) -> Result<usize, RakNetError> {
        let n = self.write(&us.to_be_bytes())?;
        if n != 2 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_unsigned_long_be(&mut self, ul: u64) -> Result<usize, RakNetError> {
        let n = self.write(&ul.to_be_bytes())?;
        if n != 8 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_fixed_string(&mut self, s: &str) -> std::result::Result<usize, RakNetError> {
        let mut n = self.write_unsigned_short_be(s.len() as u16)?;
        n += self.write(s.as_ref())?;
        if n != 2 + s.len() {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize, RakNetError> {
        for i in 0..mtu {
            let n = self.write(&[0x00])?;
            if n != 1 {
                return Err(RakNetError::TooFewBytesWritten(i as usize + n))
            }    
        }
        Ok(mtu as usize)
    }
}

trait RakNetRead {
    fn read_byte(&mut self) -> Result<u8, RakNetError>;
    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError>;
    fn read_unsigned_short_be(&mut self) -> Result<u16, RakNetError>;
    fn read_unsigned_long_be(&mut self) -> Result<u64, RakNetError>;
    fn read_fixed_string(&mut self) -> Result<String, RakNetError>;
    fn read_zero_padding(&mut self) -> Result<u16, RakNetError>;
}

impl<T> RakNetRead for T where T: Read {
    fn read_byte(&mut self) -> Result<u8, RakNetError> {
        let mut buf = vec![0u8; 1];
        let n = self.read(&mut buf)?;
        if n != 1 {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(u8::from_le_bytes(buf[0..1].try_into().unwrap()))
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError> {
        let n = self.read(buf)?;
        if n != buf.len() {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(())
    }

    fn read_unsigned_short_be(&mut self) -> Result<u16, RakNetError> {
        let mut buf = vec![0u8; 2];
        let n = self.read(&mut buf)?;
        if n != 2 {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(u16::from_be_bytes(buf[0..2].try_into().unwrap()))
    }

    fn read_unsigned_long_be(&mut self) -> Result<u64, RakNetError> {
        let mut buf = vec![0u8; 8];
        let n = self.read(&mut buf)?;
        if n != 8 {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(u64::from_be_bytes(buf[0..8].try_into().unwrap()))
    }

    fn read_fixed_string(&mut self) -> Result<String, RakNetError> {
        let length: usize = self.read_unsigned_short_be()?.into();
        let mut buf = vec![0u8; length];
        let n = self.read(&mut buf)?;
        if n != length {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        let s = String::from_utf8(buf)?;
        Ok(s)
    }

    fn read_zero_padding(&mut self) -> Result<u16, RakNetError> {
        let mut padding_length = 0u16;
        let mut buf = vec![0u8; 1];
        loop {
            let n = self.read(&mut buf)?;
            if n == 0 {
                break;
            }            
            padding_length += u16::try_from(n).unwrap(); // n should never be larger than buffer size (=1)
        }
        Ok(padding_length)
    }
}

