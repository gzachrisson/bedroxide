use std::{
    io::{self},
    fs::File,
    net::SocketAddr,
};
use simplelog::*;
use net::raknet::{RakNetError, RakNetPeer};
use log::{info};

pub mod net;

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
    let _thread = std::thread::spawn(move || peer.run());

    // Wait for ENTER to kill server
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    
    info!("Shutting down server");

    Ok(())
}
