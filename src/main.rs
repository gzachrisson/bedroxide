use std::{io, fs::File, net::SocketAddr, thread};
use simplelog::{SimpleLogger, WriteLogger, LevelFilter, Config, CombinedLogger};
use log::{info, error};

use self::net::raknet::{RakNetError, RakNetPeer, Command};

mod net;

fn main() -> Result<(), RakNetError> {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Debug, Config::default()),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    run_server()
}

fn run_server() -> Result<(), RakNetError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 19132));
    let mut peer = RakNetPeer::bind(addr)?;
    peer.set_unconnected_ping_response("MCPE;Bedroxide server;390;1.14.60;5;10;13253860892328930977;Second row;Survival;1;19132;19133;");
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
