use std::{io, fs::File, net::SocketAddr};
use simplelog::{SimpleLogger, WriteLogger, LevelFilter, Config, CombinedLogger};

use crate::{error::Result, server::Server};

mod error;
mod server;

fn main() -> Result<()> {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Debug, Config::default()),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    let server = Server::start(SocketAddr::from(([0, 0, 0, 0], 19132)))?;

    // Wait for ENTER to kill server
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    server.shutdown()?;

    Ok(())
}

