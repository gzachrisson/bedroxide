use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    io,
};

pub trait DatagramSocket {
    fn receive_datagram<'a>(&mut self, buffer: &'a mut [u8]) -> io::Result<(&'a [u8], SocketAddr)>;
    fn send_datagram<A: ToSocketAddrs>(&mut self, payload: &[u8], addr: A) -> io::Result<usize>;
}

impl DatagramSocket for UdpSocket {
    fn receive_datagram<'a>(&mut self, buf: &'a mut [u8]) -> io::Result<(&'a [u8], SocketAddr)> {
         self.recv_from(buf).map(move |(n, addr)| (&buf[..n], addr))
    }
    
    fn send_datagram<A: ToSocketAddrs>(&mut self, payload: &[u8], addr: A) -> io::Result<usize> {
         self.send_to(payload, addr)
    }
}
