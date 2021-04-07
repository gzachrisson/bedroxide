use std::{
    io::Cursor,
    net::{SocketAddr, UdpSocket, ToSocketAddrs},
    time::Duration,
};
use log::{info, error, debug};
use rand;
use crossbeam_channel::{unbounded, Sender, Receiver, Select};

use super::{
    RakNetError,
    messages::{OpenConnectionRequest1Message, UnconnectedPingMessage, UnconnectedPongMessage},
    reader::{RakNetMessageRead},
    writer::{RakNetMessageWrite},
    utils,
};

pub struct RakNetPeer
{
    socket: UdpSocket,
    unconnected_ping_response: String,
    guid: u64,
    receive_buffer: Vec<u8>,
    command_sender: Sender<Command>,
    command_receiver: Receiver<Command>,
}

pub enum Command
{
    #[allow(dead_code)]
    ProcessNow,
    StopProcessing,
}

impl RakNetPeer {
    /// Creates a RakNetPeer and binds it to a UDP socket on the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, RakNetError> {
        info!("Binding socket");
        let socket = UdpSocket::bind(addr)?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;

        info!("Listening on {}", socket.local_addr()?);
        
        let (command_sender, command_receiver) = unbounded();

        Ok(RakNetPeer {
            socket,
            unconnected_ping_response: String::new(),
            guid: rand::random(),
            receive_buffer: vec![0u8; 2048],
            command_sender,
            command_receiver
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
        // Process all received commands
        while let Ok(command) = self.command_receiver.try_recv() {
            match command
            {
                Command::ProcessNow => {}, // Processing already in progress
                Command::StopProcessing => return false,
            }
        }

        // Process all incoming packets
        loop
        {
            match self.socket.recv_from(self.receive_buffer.as_mut())
            {
                Ok((received_length, addr)) => {
                    if received_length > 0 {
                        debug!("Received {} bytes from {}: {}", received_length, addr, utils::to_hex(&self.receive_buffer[..received_length.min(40)]));

                        let payload = &self.receive_buffer[..received_length];
                        match self.process_received_packet(addr, payload)
                        {
                            Ok(_) => {}
                            Err(err) => error!("Error when processing received packet: {:?}", err),
                        }
                    } else {
                        error!("Received 0 byte message from {}", addr);
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

            // Wait for sleep_time to pass or until a command arrives
            let mut sel = Select::new();
            sel.recv(&self.command_receiver);
            match sel.ready_timeout(sleep_time)
            {
                _ => {}
            }
        }
    }    
    
    /// Sets the message returned in an unconnected ping response.
    pub fn set_unconnected_ping_response(&mut self, unconnected_ping_response: &str)
    {
        self.unconnected_ping_response = unconnected_ping_response.to_string();
    }

    /// Gets a command sender that can be used for sending commands
    /// to the processing thread once `start_processing` or
    /// `start_processing_with_duration` has been called.
    ///
    /// Use the command sender to stop the processing or
    /// to force processing to occur now.
    pub fn get_command_sender(&self) -> Sender<Command>
    {
        self.command_sender.clone()
    }

    fn process_received_packet(&self, addr: SocketAddr, payload: &[u8]) -> Result<(), RakNetError>
    {        
        let mut reader = Cursor::new(payload);
        if let Ok(ping) = UnconnectedPingMessage::read_message(&mut reader) {
            debug!("Received Unconnected Ping: time={}, client_guid={}", ping.time, ping.client_guid);
    
            let pong = UnconnectedPongMessage { time: ping.time, guid: self.guid, data: self.unconnected_ping_response.clone() };
            self.send_message(&pong, addr)?;
            debug!("Sent Unconnected Pong");
            return Ok(());
        }
        
        reader.set_position(0);
        if let Ok(pong) = UnconnectedPongMessage::read_message(&mut reader) {
            debug!("Received Unconnected Pong: time={}, guid={}, data={}", pong.time, pong.guid, pong.data);
            return Ok(());
        }

        reader.set_position(0);
        if let Ok(request1) = OpenConnectionRequest1Message::read_message(&mut reader) {
            debug!("Received Open Connection Request 1: protocol_version={}, padding_length={}", request1.protocol_version, request1.padding_length);
            return Ok(());
        }
         
        debug!("Unhandled message ID: {}", payload[0]);        
        Ok(())
    }

    fn send_message(&self, message: &dyn RakNetMessageWrite, dest: SocketAddr) -> Result<(), RakNetError> {
        let mut send_buf = Vec::with_capacity(1024); // TODO: Allocate once
        message.write_message(&mut send_buf)?;
        self.socket.send_to(&send_buf, dest)?;
        debug!("Sent {} bytes to {}: {}", send_buf.len(), dest, utils::to_hex(&send_buf[..send_buf.len().min(40)]));
        Ok(())
    }   
}
