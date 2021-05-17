use std::{
    net::{SocketAddr, UdpSocket},
    io,
};

pub trait DatagramSocket {
    fn receive_datagram<'a>(&mut self, buffer: &'a mut [u8]) -> io::Result<(&'a [u8], SocketAddr)>;
    fn send_datagram(&mut self, payload: &[u8], addr: SocketAddr) -> io::Result<usize>;
    fn local_addr(&self) -> io::Result<SocketAddr>;
}

impl DatagramSocket for UdpSocket {
    fn receive_datagram<'a>(&mut self, buf: &'a mut [u8]) -> io::Result<(&'a [u8], SocketAddr)> {
         self.recv_from(buf).map(move |(n, addr)| (&buf[..n], addr))
    }
    
    fn send_datagram(&mut self, payload: &[u8], addr: SocketAddr) -> io::Result<usize> {
         self.send_to(payload, addr)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.local_addr()
    }
}

#[cfg(test)]
use crossbeam_channel::{unbounded, Sender, Receiver, TryRecvError};

#[cfg(test)]
pub struct FakeDatagramSocket {
    receive_datagram_sender: Sender<(Vec<u8>, SocketAddr)>,
    receive_datagram_receiver: Receiver<(Vec<u8>, SocketAddr)>,
    send_datagram_sender: Sender<(Vec<u8>, SocketAddr)>,
    send_datagram_receiver: Receiver<(Vec<u8>, SocketAddr)>,
    local_addr: SocketAddr,
}

#[cfg(test)]
impl FakeDatagramSocket {
    pub fn new(local_addr: SocketAddr) -> FakeDatagramSocket {
        let (receive_datagram_sender, receive_datagram_receiver) = unbounded();
        let (send_datagram_sender, send_datagram_receiver) = unbounded();
        FakeDatagramSocket {
            receive_datagram_sender,
            receive_datagram_receiver,
            send_datagram_sender,
            send_datagram_receiver,
            local_addr,
        }
    }

    pub fn get_datagram_sender(&self) -> Sender<(Vec<u8>, SocketAddr)> {
        self.receive_datagram_sender.clone()
    }

    pub fn get_datagram_receiver(&self) -> Receiver<(Vec<u8>, SocketAddr)> {
        self.send_datagram_receiver.clone()
    }    
}

#[cfg(test)]
impl DatagramSocket for FakeDatagramSocket {
    fn receive_datagram<'a>(&mut self, buf: &'a mut [u8]) -> io::Result<(&'a [u8], SocketAddr)> {        
        match self.receive_datagram_receiver.try_recv() {
            Ok((payload, addr)) => {
                let buf_payload = &mut buf[..payload.len()];
                buf_payload.copy_from_slice(&payload);
                Ok((buf_payload, addr))
            },
            Err(TryRecvError::Empty) => Err(io::ErrorKind::WouldBlock.into()),
            _ => panic!("Received unexpected error in FakeDatagramSocket"),
        }
    }
    
    fn send_datagram(&mut self, payload: &[u8], addr: SocketAddr) -> io::Result<usize> {
        let mut buf = Vec::new();
        buf.extend_from_slice(payload);
        let buf_len = buf.len();
        self.send_datagram_sender.try_send((buf, addr))
            .map(move |_| buf_len)
            .map_err(|_| io::ErrorKind::WouldBlock.into())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.local_addr)
    }
}
