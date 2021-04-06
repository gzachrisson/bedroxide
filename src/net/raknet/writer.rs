use std::io::Write;

use super::RakNetError;

pub trait RakNetWrite {
    fn write_byte(&mut self, b: u8) -> Result<usize, RakNetError>;
    fn write_bytes(&mut self, b: &[u8]) -> Result<usize, RakNetError>;
    fn write_unsigned_short_be(&mut self, us: u16) -> Result<usize, RakNetError>;
    fn write_unsigned_long_be(&mut self, ul: u64) -> Result<usize, RakNetError>;
    fn write_fixed_string(&mut self, s: &str) -> Result<usize, RakNetError>;
    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize, RakNetError>;
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

    fn write_unsigned_long_be(&mut self, ul: u64) -> Result<usize, RakNetError> {
        let n = self.write(&ul.to_be_bytes())?;
        if n != 8 {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_fixed_string(&mut self, s: &str) -> std::result::Result<usize, RakNetError> {
        let mut n = self.write_unsigned_short_be(s.len() as u16)?;
        n += self.write(s.as_ref())?;
        if n != 2 + s.len() {
            return Err(RakNetError::TooFewBytesWritten(n))
        }
        Ok(n)
    }

    fn write_zero_padding(&mut self, mtu: u16) -> Result<usize, RakNetError> {
        for i in 0..mtu {
            let n = self.write(&[0x00])?;
            if n != 1 {
                return Err(RakNetError::TooFewBytesWritten(i as usize + n))
            }    
        }
        Ok(mtu as usize)
    }
}

pub trait RakNetMessageWrite {
    fn message_id(&self) -> u8;
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError>;
}
