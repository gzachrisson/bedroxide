use std::{io, fs::File, net::SocketAddr, thread};
use simplelog::{SimpleLogger, WriteLogger, LevelFilter, Config, CombinedLogger};
use log::{info, error};
use raknet::{Peer, PeerEvent, Command, DataWrite};

use crate::error::Result;

mod error;

fn main() -> Result<()> {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Debug, Config::default()),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    run_server()
}

fn run_server() -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 19132));
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
                    info!("Received packet from addr: {:?}, guid: {} with payload length: {}", packet.addr(), packet.guid(), packet.payload().len());
                }
                Ok(PeerEvent::SendReceiptAcked(receipt)) => {
                    info!("Received send receipt {} ACK from from addr: {:?}, guid: {}", receipt.receipt(), receipt.addr(), receipt.guid());
                }
                Ok(PeerEvent::SendReceiptLoss(receipt)) => {
                    info!("Received send receipt {} LOSS from from addr: {:?}, guid: {}", receipt.receipt(), receipt.addr(), receipt.guid());
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
    let processing_thread = thread::spawn(move || peer.start_processing());    

    // Wait for ENTER to kill server
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    
    info!("Shutting down server");

    command_sender.send(Command::StopProcessing)?;

    match event_receiver_thread.join() {
        Ok(()) => info!("Event receiver thread stopped"),
        Err(err) => error!("Could not stop event receiver thread: {:?}", err)
    }

    match processing_thread.join() {
        Ok(()) => info!("Server stopped"),
        Err(err) => error!("Could not stop server: {:?}", err)
    }

    Ok(())
}
