use std::{io::{Cursor, Read}, net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6}, usize};

use crate::{error::{Error, ReadError, Result}, number::u24};

pub trait DataRead {
    fn read_u8(&mut self) -> Result<u8>;
    fn read_u8_and_compare(&mut self, data: u8) -> Result<()>;
    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()>;
    fn read_bytes_to_boxed_slice(&mut self, length: usize) -> Result<Box<[u8]>>;
    fn read_bytes_to_end(&mut self, buf: &mut Vec<u8>) -> Result<()>;
    fn read_bytes_and_compare(&mut self, data: &[u8]) -> Result<()>;
    fn read_u16(&mut self) -> Result<u16>;
    fn read_u16_be(&mut self) -> Result<u16>;
    fn read_u24(&mut self) -> Result<u24>;
    fn read_u32(&mut self) -> Result<u32>;
    fn read_u32_be(&mut self) -> Result<u32>;
    fn read_u64_be(&mut self) -> Result<u64>;
    fn read_f32_be(&mut self) -> Result<f32>;
    fn read_fixed_string(&mut self) -> Result<String>;
    fn read_zero_padding(&mut self) -> Result<u16>;
    fn read_socket_addr(&mut self) -> Result<SocketAddr>;
    fn has_more(&self) -> bool;
}

pub struct DataReader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> DataReader<'a> {
    pub fn new(data: &'a [u8]) -> DataReader<'a> {
        DataReader {
            cursor: Cursor::new(data),
        }
    }
}

impl<'a> DataRead for DataReader<'a> {
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.cursor.read_exact(&mut buf)?;
        Ok(u8::from_le_bytes(buf))
    }

    fn read_u8_and_compare(&mut self, data: u8) -> Result<()> {
        let byte = self.read_u8()?;
        if byte == data {
            Ok(())
        } else {
            Err(Error::ReadError(ReadError::CompareFailed))
        }
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        self.cursor.read_exact(buf)?;
        Ok(())
    }

    fn read_bytes_to_boxed_slice(&mut self, length: usize) -> Result<Box<[u8]>> {
        if length > self.cursor.get_ref().len() - self.cursor.position() as usize {
            return Err(ReadError::NotAllBytesRead(self.cursor.get_ref().len() - self.cursor.position() as usize).into());
        }
        let boxed_slice = self.cursor.get_ref()[self.cursor.position() as usize..self.cursor.position() as usize + length].to_vec().into_boxed_slice();
        self.cursor.set_position(self.cursor.position() + length as u64);
        Ok(boxed_slice)
    }

    fn read_bytes_to_end(&mut self, buf: &mut Vec<u8>) -> Result<()> {        
        self.cursor.read_to_end(buf)?;
        Ok(())
    }

    fn read_bytes_and_compare(&mut self, data: &[u8]) -> Result<()> {
        let mut buf = vec![0u8; data.len()];
        self.read_bytes(&mut buf)?;
        if buf == data {
            Ok(())
        } else {
            Err(Error::ReadError(ReadError::CompareFailed))
        }
    }

    fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_bytes(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u16_be(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_bytes(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u24(&mut self) -> Result<u24> {
        let mut buf = [0u8; 3];
        self.read_bytes(&mut buf)?;
        Ok(u24::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_bytes(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u32_be(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_bytes(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64_be(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.read_bytes(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_f32_be(&mut self) -> Result<f32> {
        let mut buf = [0u8; 4];
        self.read_bytes(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    fn read_fixed_string(&mut self) -> Result<String> {
        let length: usize = self.read_u16_be()?.into();
        let mut buf = vec![0u8; length];
        self.read_bytes(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }

    fn read_zero_padding(&mut self) -> Result<u16> {
        let mut padding_length = 0u16;
        let mut buf = [0u8; 1];
        loop {
            let n = self.cursor.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if padding_length == u16::MAX {
                return Err(ReadError::TooLongZeroPadding.into());
            }
            padding_length += 1;
        }
        Ok(padding_length)
    }

    fn read_socket_addr(&mut self) -> Result<SocketAddr> {
        let ip_version = self.read_u8()?;
        match ip_version {
            0x04 => {
                let mut ip = [0u8; 4];                
                self.cursor.read_exact(&mut ip)?;
                let port = self.read_u16_be()?;
                Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(!ip[0], !ip[1], !ip[2], !ip[3])), port))
            },
            0x06 => {
                let _ = self.read_u16()?; // family
                let port = self.read_u16_be()?;
                let flowinfo = self.read_u32()?;
                let mut ip = [0u8; 16];
                self.cursor.read_exact(&mut ip)?;
                let scope_id = self.read_u32()?;
                Ok(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(ip), port, flowinfo, scope_id)))
            },
            _ => Err(ReadError::InvalidIpVersion.into()),
        }
    }

    fn has_more(&self) -> bool {
        (self.cursor.position() as usize) < self.cursor.get_ref().len()
    }
}

pub trait OfflineMessageRead: Sized {
    /// Reads a message including the message identifier.
    /// 
    /// This function assumes security is disabled on our peer, or
    /// that the security state can be determined from the message content.
    fn read_message(reader: &mut dyn DataRead) -> Result<Self>;

    /// Reads a message including the message identifier assuming
    /// security is enabled on our peer.
    /// The default implementation if not overridden just calls `read_message()`.
    fn read_message_with_security(reader: &mut dyn DataRead) -> Result<Self> {
        Self::read_message(reader)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use crate::DataRead;
    use super::DataReader;

    #[test]
    fn read_socket_addr_ipv4() {
        // Arrange
        let buf = vec![0x04u8, !192, !168, !1, !248, 0x12, 0x34];
        let mut reader = DataReader::new(&buf);
        
        // Act
        let socket_addr = reader.read_socket_addr().expect("Could not read SocketAddr");

        // Assert
        assert_eq!(SocketAddr::from(([192, 168, 1, 248], 0x1234)), socket_addr);
    }

    #[test]
    fn read_socket_addr_ipv6() {
        // Arrange
        let buf = vec![
            6u8, // IP version = 6
            0x18, 0x00, // sin6_family (little endian): 0x0018=24=AF_INET6
            0x12, 0x34, // sin6_port (big endian): 0x1234
            0x78, 0x56, 0x34, 0x12, // sin6_flowinfo (little endian): 0x12345678
            0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0xe0, 0x05, 0x63, 0xd8, 0x39, 0x49, // sin6_addr: fe80::8:e005:63d8:3949
            0x44, 0x33, 0x22, 0x11, // sin6_scope_id (little endian): 0x11223344
            ];
        let mut reader = DataReader::new(&buf);
        
        // Act
        let socket_addr = reader.read_socket_addr().expect("Could not read SocketAddr");

        // Assert
        if let SocketAddr::V6(socket_addr_v6) = socket_addr {
            assert_eq!(0x1234, socket_addr_v6.port());
            assert_eq!(0x12345678, socket_addr_v6.flowinfo());
            assert_eq!([0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0xe0, 0x05, 0x63, 0xd8, 0x39, 0x49], socket_addr_v6.ip().octets());
            assert_eq!(0x11223344, socket_addr_v6.scope_id());
        } else { 
            panic!("Did not receive IP V6");
        }
    }    
}