use std::{net::SocketAddr, thread};
use log::{debug, error, info};
use raknet::{channel::Sender, Peer, PeerEvent, Command, DataWrite};

use crate::error::Result;

pub struct Server {
    raknet_thread: thread::JoinHandle<()>,
    event_receiver_thread: thread::JoinHandle<()>,
    command_sender: Sender<Command>,
}

impl Server {
    pub fn start(addr: SocketAddr) -> Result<Self> {
        let mut peer = Peer::bind(addr)?;
        let mut ping_response = Vec::new();
        ping_response.write_fixed_string("MCPE;Bedroxide server;390;1.14.60;5;10;13253860892328930977;Second row;Survival;1;19132;19133;").expect("Could not write ping response");
        peer.set_offline_ping_response(ping_response);
        let command_sender = peer.command_sender();
        let event_receiver = peer.event_receiver();
        let event_receiver_thread = thread::spawn(move || {
            loop {
                match event_receiver.recv() {
                    Ok(PeerEvent::Packet(packet)) => {
                        debug!("Received packet from addr: {:?}, guid: {} with payload length: {}", packet.addr(), packet.guid(), packet.payload().len());
                    }
                    Ok(PeerEvent::SendReceiptAcked(receipt)) => {
                        debug!("Received send receipt {} ACK from from addr: {:?}, guid: {}", receipt.receipt(), receipt.addr(), receipt.guid());
                    }
                    Ok(PeerEvent::SendReceiptLoss(receipt)) => {
                        debug!("Received send receipt {} LOSS from from addr: {:?}, guid: {}", receipt.receipt(), receipt.addr(), receipt.guid());
                    }
                    Ok(PeerEvent::IncomingConnection(connection)) => {
                        info!("Incoming connection on addr: {:?}, guid: {}", connection.addr(), connection.guid());
                    }       
                    Err(_) => {
                        info!("Stopping event receiver thread");
                        break;
                    }
                }
            }
        });        
        let raknet_thread = thread::spawn(move || peer.start_processing());
        Ok(Server {
            raknet_thread,
            event_receiver_thread,
            command_sender,
        })
    }

    pub fn shutdown(self) -> Result<()> {
        info!("Shutting down server");
    
        self.command_sender.send(Command::StopProcessing)?;
    
        match self.event_receiver_thread.join() {
            Ok(()) => info!("Event receiver thread stopped"),
            Err(err) => error!("Could not stop event receiver thread: {:?}", err)
        }
    
        match self.raknet_thread.join() {
            Ok(()) => info!("Server stopped"),
            Err(err) => error!("Could not stop server: {:?}", err)
        }
    
        Ok(())
    }
   
}