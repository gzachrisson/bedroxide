use std::{io, fs::File, net::SocketAddr, thread};
use simplelog::{SimpleLogger, WriteLogger, LevelFilter, Config, CombinedLogger};
use log::{info, error};
use raknet::{RakNetPeer, Command, RakNetWrite};

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
    let mut peer = RakNetPeer::bind(addr)?;
    let mut ping_response = Vec::new();
    ping_response.write_fixed_string("MCPE;Bedroxide server;390;1.14.60;5;10;13253860892328930977;Second row;Survival;1;19132;19133;").expect("Could not write ping response");
    peer.set_offline_ping_response(ping_response);
    let command_sender = peer.get_command_sender();

    let processing_thread = thread::spawn(move || peer.start_processing());

    // Wait for ENTER to kill server
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    
    info!("Shutting down server");

    command_sender.send(Command::StopProcessing)?;

    match processing_thread.join()
    {
        Ok(()) => info!("Server stopped"),
        Err(err) => error!("Could not stop server: {:?}", err)
    }

    Ok(())
}
