use std::{
    net::{UdpSocket, ToSocketAddrs},
    time::Duration,
};
use log::{info};
use crossbeam_channel::{unbounded, Sender, Receiver, Select};

use crate::{
    config::Config,
    connection_manager::ConnectionManager,
    RakNetError,
};

pub struct RakNetPeer
{
    connection_manager: ConnectionManager<UdpSocket>,
    command_sender: Sender<Command>,
    command_receiver: Receiver<Command>,
}

/// Commands that can sent over the command sender
/// received from `get_command_sender`.
/// The commands are only executed after
/// `start_processing` or `start_processing_with_duration`
/// has been called.
pub enum Command
{
    #[allow(dead_code)]
    /// Releases the processing thread if it is sleeping
    /// so it will process incoming/outgoing messages
    /// immediately.
    ProcessNow,
    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 bytes it will be truncated.
    /// This does the same as the `set_offline_ping_response` method.
    SetOfflinePingResponse(Vec<u8>),
    /// Stops the processing loop.
    /// Use this to make `start_processing` and
    /// `start_processing_with_duration` return.
    StopProcessing,
}

impl RakNetPeer {
    /// Creates a RakNetPeer with a default `Config` and binds it to
    /// a UDP socket on the specified address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, RakNetError> {
        Self::bind_with_config(addr, Config::default())
    }

    /// Creates a RakNetPeer with the specified `Config` and binds it to
    /// a UDP socket on the specified address.
    pub fn bind_with_config<A: ToSocketAddrs>(addr: A, config: Config) -> Result<Self, RakNetError> {
        info!("Binding socket");
        let socket = UdpSocket::bind(addr)?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;

        info!("Listening on {}", socket.local_addr()?);

        let (command_sender, command_receiver) = unbounded();
        Ok(RakNetPeer {
            connection_manager: ConnectionManager::new(socket, config),
            command_sender,
            command_receiver,           
        })
    }

    /// Sends and receives packages/events and updates connections.
    /// 
    /// Use `process` to manually decide when to process network
    /// events. For an automatic processing loop use `start_processing`
    /// or `start_processing_with_duration` instead.
    pub fn process(&mut self) {
        self.connection_manager.process();
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
            // Process all network packages and events
            self.process();
            
            // Wait for sleep_time to pass or until a command arrives
            let mut sel = Select::new();
            sel.recv(&self.command_receiver);
            match sel.ready_timeout(sleep_time) {
                _ => {}
            }

            // Perform all received commands
            while let Ok(command) = self.command_receiver.try_recv() {
                match command
                {
                    Command::ProcessNow => {},
                    Command::SetOfflinePingResponse(ping_response) =>
                        self.connection_manager.set_offline_ping_response(ping_response),
                    Command::StopProcessing => return,
                }
            }
        }
    }    
    
    /// Sets the response returned to an offline ping packet.
    /// If the response is longer than 399 bytes it will be truncated.
    pub fn set_offline_ping_response(&mut self, ping_response: Vec<u8>)
    {
        self.connection_manager.set_offline_ping_response(ping_response);
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
}
