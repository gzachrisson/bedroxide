use async_std::net::UdpSocket;
use async_std::task;
use log::{info, error, debug};
use simplelog::*;
use std::fs::File;
use std::io::{Write, Read, Cursor};
use rand;
use std::convert::TryInto;

fn main() -> Result<(), RakNetError> {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Debug, Config::default()),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    task::block_on(run_server(19132))
}

#[derive(Debug)]
enum RakNetError {
    IoError(std::io::Error),
    TooFewBytesWritten(usize),
    TooFewBytesRead(usize)
}

impl From<std::io::Error> for RakNetError {
    fn from(error: std::io::Error) -> Self {
        RakNetError::IoError(error)
    }
}

trait RakNetWrite {
    fn write_byte(&mut self, b: u8) -> Result<usize, RakNetError>;
    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, RakNetError>;
    fn write_unsigned_short_be(&mut self, b: u16) -> Result<usize, RakNetError>;
    fn write_unsigned_long(&mut self, l: u64) -> Result<usize, RakNetError>;
    fn write_string(&mut self, s: &str) -> Result<usize, RakNetError>;
}

impl<T> RakNetWrite for T where T: Write {
    fn write_byte(&mut self, b: u8) -> Result<usize, RakNetError> {
        let n = self.write(&[b])?;
        if n != 1 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, RakNetError> {
        let n = self.write(b)?;
        if n != b.len() {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_unsigned_short_be(&mut self, us: u16) -> Result<usize, RakNetError> {
        let n = self.write(&us.to_be_bytes())?;
        if n != 2 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_unsigned_long(&mut self, ul: u64) -> Result<usize, RakNetError> {
        let n = self.write(&ul.to_le_bytes())?;
        if n != 8 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_string(&mut self, s: &str) -> std::result::Result<usize, RakNetError> {
        let mut n = self.write_unsigned_short_be(s.len() as u16)?;
        n += self.write(s.as_ref())?;
        if n != 2 + s.len() {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }
}

trait RakNetRead {
    fn read_byte(&mut self) -> Result<u8, RakNetError>;
    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError>;
    fn read_unsigned_long(&mut self) -> Result<u64, RakNetError>;
}

impl<T> RakNetRead for T where T: Read {
    fn read_byte(&mut self) -> Result<u8, RakNetError> {
        let mut buf = vec![0u8; 1];
        let n = self.read(&mut buf)?;
        if n != 1 {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(u8::from_le_bytes(buf[0..1].try_into().unwrap()))
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError> {
        let n = self.read(buf)?;
        if n != buf.len() {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(())
    }

    fn read_unsigned_long(&mut self) -> std::result::Result<u64, RakNetError> {
        let mut buf = vec![0u8; 8];
        let n = self.read(&mut buf)?;
        if n != 8 {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        Ok(u64::from_le_bytes(buf[0..8].try_into().unwrap()))
    }
}

async fn run_server(port: u16) -> Result<(), RakNetError> {    
    info!("Bedroxide server starting...");
    let inaddr_any = "0.0.0.0";
    let socket = UdpSocket::bind((inaddr_any, port)).await?;
    socket.set_broadcast(true)?;
    let mut buf = vec![0u8; 1024];
    let motd = "MCPE;Bedroxide server;390;1.14.60;5;10;13253860892328930977;Second row;Survival;1;19132;19133;";
    let magic: [u8; 16] = [0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78];
    let server_guid: u64 = rand::random();

    info!("Listening on {}", socket.local_addr()?);

    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        if n == 0 {
            error!("Received 0 byte package from {}", peer);
            continue;
        }
        debug!("Received {} bytes from {}: {}", n, peer, to_hex(&buf, n.min(40)));
        let mut reader = Cursor::new(&buf);
        let packet_id = reader.read_byte()?;
        // Unconnected Ping
        if packet_id == 0x01 {
            debug!("  Received Unconnected Ping");
            let time = reader.read_unsigned_long()?;
            let mut magic_buf = vec![0u8; 16];
            reader.read_bytes(&mut magic_buf)?;
            let _client_guid = reader.read_unsigned_long()?;

            // Send Unconnected Pong
            let mut send_buf = Vec::with_capacity(1024);
            send_buf.write_byte(0x1c)?; // Unconnected Pong
            send_buf.write_unsigned_long(time)?;
            send_buf.write_unsigned_long(server_guid)?;
            send_buf.write_bytes(&magic)?;
            send_buf.write_string(motd)?;

            socket.send_to(&send_buf, peer).await?;
            debug!("Sent {} bytes to {}: {}", send_buf.len(), peer, to_hex(&buf, send_buf.len().min(40)));
            debug!("  Sent Unconnected Pong");
        }
    }
}

fn to_hex(buf: &Vec<u8>, n: usize) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for &byte in buf.iter().take(n) {
        write!(&mut s, "{:02X} ", byte).expect("Unable to write");
    }
    return s;
}