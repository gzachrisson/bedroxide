use std::{
    fs::File,
    net::SocketAddr,
};

use async_std::task;
use simplelog::*;
use net::raknet::{RakNetError, RakNetServer};

pub mod net;

fn main() -> Result<(), RakNetError> {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Debug, Config::default()),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    task::block_on(run_server())
}

async fn run_server() -> Result<(), RakNetError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 19132));
    let mut server = RakNetServer::bind(addr).await?;
    server.run().await?;
    Ok(())
}
