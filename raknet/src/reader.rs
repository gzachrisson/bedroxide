use std::{
    convert::{TryFrom, TryInto},
    io::Read,
};

use super::RakNetError;

pub trait RakNetRead {
    fn read_byte(&mut self) -> Result<u8, RakNetError>;
    fn read_byte_and_compare(&mut self, data: u8) -> Result<(), RakNetError>;
    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError>;
    fn read_bytes_and_compare(&mut self, data: &[u8]) -> Result<(), RakNetError>;
    fn read_unsigned_short_be(&mut self) -> Result<u16, RakNetError>;
    fn read_unsigned_long_be(&mut self) -> Result<u64, RakNetError>;
    fn read_fixed_string(&mut self) -> Result<String, RakNetError>;
    fn read_zero_padding(&mut self) -> Result<u16, RakNetError>;
}

impl<T> RakNetRead for T where T: Read {
    fn read_byte(&mut self) -> Result<u8, RakNetError> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(u8::from_le_bytes(buf[0..1].try_into().unwrap()))
    }

    fn read_byte_and_compare(&mut self, data: u8) -> Result<(), RakNetError> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        if buf[0] == data {
            Ok(())
        } else {
            Err(RakNetError::InvalidData)
        }
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), RakNetError> {
        self.read_exact(buf)?;
        Ok(())
    }

    fn read_bytes_and_compare(&mut self, data: &[u8]) -> Result<(), RakNetError> {
        let mut buf = vec![0u8; data.len()];
        self.read_exact(&mut buf)?;
        if buf == data {
            Ok(())
        } else {
            Err(RakNetError::InvalidData)
        }
    }

    fn read_unsigned_short_be(&mut self) -> Result<u16, RakNetError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf[0..2].try_into().unwrap()))
    }

    fn read_unsigned_long_be(&mut self) -> Result<u64, RakNetError> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf[0..8].try_into().unwrap()))
    }

    fn read_fixed_string(&mut self) -> Result<String, RakNetError> {
        let length: usize = self.read_unsigned_short_be()?.into();
        let mut buf = vec![0u8; length];
        let n = self.read(&mut buf)?;
        if n != length {
            return Err(RakNetError::TooFewBytesRead(n))
        }
        let s = String::from_utf8(buf)?;
        Ok(s)
    }

    fn read_zero_padding(&mut self) -> Result<u16, RakNetError> {
        let mut padding_length = 0u16;
        let mut buf = [0u8; 1];
        loop {
            let n = self.read(&mut buf)?;
            if n == 0 {
                break;
            }            
            padding_length += u16::try_from(n).unwrap(); // n should never be larger than buffer size (=1)
        }
        Ok(padding_length)
    }
}

pub trait RakNetMessageRead: Sized {
    /// Reads a message including the message identifier.
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError>;
}
