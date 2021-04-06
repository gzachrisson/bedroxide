use super::{
    RakNetError,
    reader::{RakNetRead, RakNetMessageRead},
    writer::{RakNetWrite, RakNetMessageWrite},
};

pub struct UnconnectedPingMessage {
    pub time: u64,
    pub client_guid: u64
}

impl RakNetMessageRead for UnconnectedPingMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        let time = reader.read_unsigned_long_be()?;
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let client_guid = reader.read_unsigned_long_be()?;
        Ok(UnconnectedPingMessage { time, client_guid })
    }
}

impl RakNetMessageWrite for UnconnectedPingMessage {
    fn message_id(&self) -> u8 { 0x01 }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_unsigned_long_be(self.time)?;
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_unsigned_long_be(self.client_guid)?;
        Ok(())
    }
}

pub struct UnconnectedPongMessage {
    pub time: u64,
    pub guid: u64,
    pub data: String
}

impl RakNetMessageRead for UnconnectedPongMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        let time = reader.read_unsigned_long_be()?;
        let guid = reader.read_unsigned_long_be()?;
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let data = reader.read_fixed_string()?;
        Ok(UnconnectedPongMessage { time, guid, data })
    }
}

impl RakNetMessageWrite for UnconnectedPongMessage {
    fn message_id(&self) -> u8 { 0x1c }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_unsigned_long_be(self.time)?;
        writer.write_unsigned_long_be(self.guid)?;
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_fixed_string(&self.data)?;
        Ok(())      
    }
}

pub struct OpenConnectionRequest1Message {
    pub protocol_version: u8,
    pub padding_length: u16
}

impl RakNetMessageRead for OpenConnectionRequest1Message {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_bytes(&mut [0u8; 16])?; // Offline Message ID = 00ffff00fefefefefdfdfdfd12345678
        let protocol_version = reader.read_byte()?;
        let padding_length = reader.read_zero_padding()?;
        Ok(OpenConnectionRequest1Message { protocol_version, padding_length })
    }
}

impl RakNetMessageWrite for OpenConnectionRequest1Message {
    fn message_id(&self) -> u8 { 0x05 }

    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_bytes(&[0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78])?; // Offline Message ID
        writer.write_byte(self.protocol_version)?;
        writer.write_zero_padding(self.padding_length)?;
        Ok(())      
    }
}
