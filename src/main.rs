use std::{io, fs::File, net::SocketAddr};
use simplelog::{SimpleLogger, WriteLogger, LevelFilter, Config, CombinedLogger};
use log::{info};

use self::net::raknet::{RakNetError, RakNetPeer};

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
    
    let _thread = std::thread::spawn(move || peer.start_processing());

    // Wait for ENTER to kill server
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    
    info!("Shutting down server");

    Ok(())
}
