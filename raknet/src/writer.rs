use std::{
    io::Write,
    net::SocketAddr,
};

use crate::{Result, u24, WriteError};

pub trait RakNetWrite {
    fn write_u8(&mut self, b: u8) -> Result<usize>;
    fn write_bytes(&mut self, b: &[u8]) -> Result<usize>;
    fn write_u16(&mut self, us: u16) -> Result<usize>;
    fn write_u16_be(&mut self, us: u16) -> Result<usize>;
    fn write_u24(&mut self, value: u24) -> Result<usize>;
    fn write_u32(&mut self, u: u32) -> Result<usize>;
    fn write_u32_be(&mut self, u: u32) -> Result<usize>;
    fn write_u64_be(&mut self, ul: u64) -> Result<usize>;
    fn write_f32_be(&mut self, value: f32) -> Result<usize>;
    fn write_fixed_string(&mut self, s: &str) -> Result<usize>;
    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize>;
    fn write_socket_addr(&mut self, addr: &SocketAddr) -> Result<usize>;
}

impl<T> RakNetWrite for T where T: Write {
    fn write_u8(&mut self, b: u8) -> Result<usize> {
        let n = self.write(&[b])?;
        if n != 1 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_bytes(&mut self, b: &[u8]) -> Result<usize> {
        let n = self.write(b)?;
        if n != b.len() {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_u16(&mut self, us: u16) -> Result<usize> {
        let n = self.write(&us.to_le_bytes())?;
        if n != 2 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_u16_be(&mut self, us: u16) -> Result<usize> {
        let n = self.write(&us.to_be_bytes())?;
        if n != 2 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_u24(&mut self, value: u24) -> Result<usize> {
        let n = self.write(&value.to_le_bytes())?;
        if n != 3 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }    

    fn write_u32(&mut self, u: u32) -> Result<usize> {
        let n = self.write(&u.to_le_bytes())?;
        if n != 4 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }    

    fn write_u32_be(&mut self, u: u32) -> Result<usize> {
        let n = self.write(&u.to_be_bytes())?;
        if n != 4 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }    

    fn write_u64_be(&mut self, ul: u64) -> Result<usize> {
        let n = self.write(&ul.to_be_bytes())?;
        if n != 8 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_f32_be(&mut self, value: f32) -> Result<usize> {
        let n = self.write(&value.to_be_bytes())?;
        if n != 4 {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_fixed_string(&mut self, s: &str) -> Result<usize> {
        let mut n = self.write_u16_be(s.len() as u16)?;
        n += self.write(s.as_ref())?;
        if n != 2 + s.len() {
            return Err(WriteError::NotAllBytesWritten(n).into())
        }
        Ok(n)
    }

    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize> {
        for i in 0..mtu {
            let n = self.write(&[0x00])?;
            if n != 1 {
                return Err(WriteError::NotAllBytesWritten(i as usize + n).into())
            }    
        }
        Ok(mtu as usize)
    }

    fn write_socket_addr(&mut self, addr: &SocketAddr) -> Result<usize> {
        match addr {
            SocketAddr::V4(addr_v4) => {
                let mut n = self.write_u8(4)?;
                let mut ip = addr_v4.ip().octets();
                // Bitwise invert the bytes
                for i in 0..ip.len() {
                    ip[i] = !ip[i];
                }
                n += self.write_bytes(&ip)?;
                n += self.write_u16_be(addr_v4.port())?;
                Ok(n)
            },
            SocketAddr::V6(addr_v6) => {
                let mut n = self.write_u8(6)?;
                n += self.write_u16(24)?; // family (little endian): 24=AF_INET6
                n += self.write_u16_be(addr_v6.port())?;
                n += self.write_u32(addr_v6.flowinfo())?;
                n += self.write_bytes(&addr_v6.ip().octets())?;
                n += self.write_u32(addr_v6.scope_id())?;
                Ok(n)
            }
        }
    }    
}

pub trait RakNetMessageWrite {
    /// Writes a message including the message identifier.
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};

    use crate::writer::RakNetWrite;

    #[test]
    fn write_socket_addr_ipv4() {
        // Arrange
        let socket_addr = SocketAddr::from(([192, 168, 1, 248], 0x1234));
        let mut buf = Vec::new();

        // Act
        let bytes_written = buf.write_socket_addr(&socket_addr).expect("Could not write SocketAddr");

        // Assert
        assert_eq!(7, bytes_written);
        assert_eq!(vec![0x04u8, !192, !168, !1, !248, 0x12, 0x34], buf);
    }

    #[test]
    fn write_socket_addr_ipv6() {
        // Arrange
        let ip = [0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0xe0, 0x05, 0x63, 0xd8, 0x39, 0x49];
        let port = 0x1234;
        let flowinfo = 0x12345678;
        let scope_id = 0x11223344;
        let socket_addr = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(ip), port, flowinfo, scope_id));
        let mut buf = Vec::new();
        
        // Act
        let bytes_written = buf.write_socket_addr(&socket_addr).expect("Could not write SocketAddr");

        // Assert
        assert_eq!(29, bytes_written);
        assert_eq!(vec![
            6u8, // IP version = 6
            0x18, 0x00, // sin6_family (little endian): 0x0018=24=AF_INET6
            0x12, 0x34, // sin6_port (big endian): 0x1234
            0x78, 0x56, 0x34, 0x12, // sin6_flowinfo (little endian): 0x12345678
            0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0xe0, 0x05, 0x63, 0xd8, 0x39, 0x49, // sin6_addr: fe80::8:e005:63d8:3949
            0x44, 0x33, 0x22, 0x11, // sin6_scope_id (little endian): 0x11223344
            ], buf);
    }    
}