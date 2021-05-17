use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crossbeam_channel::Sender;
use log::error;

use crate::{Config, PeerEvent, Result, constants::MAX_NUMBER_OF_INTERNAL_IDS, socket::DatagramSocket};

pub struct Communicator<T: DatagramSocket> {
    config: Config,
    socket: T,
    event_sender: Sender<PeerEvent>,
}

impl<T: DatagramSocket> Communicator<T> {
    pub fn new(socket: T, config: Config, event_sender: Sender<PeerEvent>) -> Self {
        Communicator {
            config,
            socket,
            event_sender,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn socket(&mut self) -> &mut T {
        &mut self.socket
    }

    pub fn send_datagram(&mut self, payload: &[u8], addr: SocketAddr) {
        if let Err(err) = self.socket.send_datagram(payload, addr) {
            error!("Failed sending datagram to {}: {:?}", addr, err);
        }
    }

    pub fn send_event(&mut self, event: PeerEvent) {
        if let Err(_) = self.event_sender.send(event) {
            error!("Send event failed since the event receiver has been dropped");
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    pub fn get_addr_list(&self) -> [SocketAddr; MAX_NUMBER_OF_INTERNAL_IDS] {
        let mut addr_list = [SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0); MAX_NUMBER_OF_INTERNAL_IDS];
        if let Ok(local_addr) = self.local_addr() {
            addr_list[0] = local_addr;
        }
        addr_list
    }
}