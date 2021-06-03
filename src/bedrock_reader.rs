use std::io::{Cursor, Read};

use crate::error::{Error, Result};

pub struct BedrockReader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> BedrockReader<'a> {
    pub fn new(data: &'a [u8]) -> BedrockReader<'a> {
        BedrockReader {
            cursor: Cursor::new(data),
        }
    }
    
    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_bytes_as_slice(&mut self, length: usize) -> Result<&[u8]> {
        let pos = self.cursor.position() as usize;
        if length > self.cursor.get_ref().len() - pos {
            return Err(Error::NotAllBytesRead);
        }
        self.cursor.set_position((pos + length) as u64);
        Ok(&self.cursor.get_ref()[pos..pos + length])
    }    

    pub fn read_varint_to_u32(&mut self) -> Result<u32> {
        let mut result: u32 = 0;
        let mut bits = 0;
        loop {
            let b = self.read_u8()?;
            result = result | (((b & 0x7f) as u32) << bits);
            bits = bits + 7;
            if bits > 32 {
                return Err(Error::VarIntTooLarge)
            }
            if b & 0x80 == 0 {
                break;
            }
        }
        Ok(result)
    }

    pub fn has_more(&self) -> bool {
        (self.cursor.position() as usize) < self.cursor.get_ref().len() - 1
    }
}