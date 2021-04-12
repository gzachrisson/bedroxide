use crate::{
    constants::OFFLINE_MESSAGE_ID,
    error::RakNetError,
    message_ids::MessageId,
    reader::{RakNetRead, RakNetMessageRead},
    writer::{RakNetWrite, RakNetMessageWrite},
};

pub struct UnconnectedPingMessage {
    pub time: u64,
    pub client_guid: u64,
}

impl RakNetMessageRead for UnconnectedPingMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_byte_and_compare(MessageId::UnconnectedPing.into())?;
        let time = reader.read_unsigned_long_be()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let client_guid = reader.read_unsigned_long_be()?;
        Ok(UnconnectedPingMessage { time, client_guid })
    }
}

impl RakNetMessageWrite for UnconnectedPingMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_byte(MessageId::UnconnectedPing.into())?;
        writer.write_unsigned_long_be(self.time)?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_unsigned_long_be(self.client_guid)?;
        Ok(())
    }
}

pub struct UnconnectedPongMessage {
    pub time: u64,
    pub guid: u64,
    pub data: Vec<u8>,
}

impl RakNetMessageRead for UnconnectedPongMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_byte_and_compare(MessageId::UnconnectedPong.into())?;
        let time = reader.read_unsigned_long_be()?;
        let guid = reader.read_unsigned_long_be()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let mut data = Vec::new();
        reader.read_bytes_to_end(&mut data)?;
        Ok(UnconnectedPongMessage { time, guid, data })
    }
}

impl RakNetMessageWrite for UnconnectedPongMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_byte(MessageId::UnconnectedPong.into())?;
        writer.write_unsigned_long_be(self.time)?;
        writer.write_unsigned_long_be(self.guid)?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_bytes(&self.data)?;
        Ok(())      
    }
}

pub struct OpenConnectionRequest1Message {
    pub protocol_version: u8,
    pub padding_length: u16,
}

impl RakNetMessageRead for OpenConnectionRequest1Message {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_byte_and_compare(MessageId::OpenConnectionRequest1.into())?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let protocol_version = reader.read_byte()?;
        let padding_length = reader.read_zero_padding()?;
        Ok(OpenConnectionRequest1Message { protocol_version, padding_length })
    }
}

impl RakNetMessageWrite for OpenConnectionRequest1Message {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_byte(MessageId::OpenConnectionRequest1.into())?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_byte(self.protocol_version)?;
        writer.write_zero_padding(self.padding_length)?;
        Ok(())      
    }
}

pub struct IncompatibleProtocolVersionMessage {
    pub protocol_version: u8,
    pub guid: u64,
}

impl RakNetMessageRead for IncompatibleProtocolVersionMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_byte_and_compare(MessageId::IncompatibleProtocolVersion.into())?;
        let protocol_version = reader.read_byte()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let guid = reader.read_unsigned_long_be()?;
        Ok(IncompatibleProtocolVersionMessage { protocol_version, guid })
    }
}

impl RakNetMessageWrite for IncompatibleProtocolVersionMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_byte(MessageId::IncompatibleProtocolVersion.into())?;
        writer.write_byte(self.protocol_version)?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_unsigned_long_be(self.guid)?;
        Ok(())      
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        error::RakNetError,
        messages::{UnconnectedPingMessage, UnconnectedPongMessage, OpenConnectionRequest1Message, IncompatibleProtocolVersionMessage},
        reader::RakNetMessageRead,
        writer::RakNetMessageWrite,
    };

    #[test]
    fn read_unconnected_ping() {
        // Arrange
        let buf = vec![
            0x01, // Message ID: Unconnected ping
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Client guid: 0x8877665544332211
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let ping = UnconnectedPingMessage::read_message(&mut reader).expect("Failed to read unconnected ping");

        // Assert
        assert_eq!(0x0123456789ABCDEF, ping.time);
        assert_eq!(0x8877665544332211, ping.client_guid);
    }

    #[test]
    fn read_unconnected_ping_invalid_offline_message_id() {
        // Arrange
        let buf = vec![
            0x01, // Message ID: Unconnected ping
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // INVALID Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Client guid: 0x8877665544332211
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let result = UnconnectedPingMessage::read_message(&mut reader);

        // Assert
        match result {
            Ok(_) => panic!("Message read even though offline message ID was incorrect"),
            Err(RakNetError::InvalidData) => {},
            _ => panic!("Invalid error reading message with invalid message ID"),
        }
    }    

    #[test]
    fn write_unconnected_ping() {
        // Arrange
        let ping = UnconnectedPingMessage {
            time: 0x0123456789ABCDEF,
            client_guid: 0x8877665544332211,
        };
        let mut buf = Vec::new();

        // Act
        ping.write_message(&mut buf).expect("Could not write ping message");

        // Assert
        assert_eq!(vec![
            0x01, // Message ID: Unconnected ping
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Client guid: 0x8877665544332211
        ],
        buf);

    }

    #[test]
    fn read_unconnected_pong() {
        // Arrange
        let buf = vec![
            0x1C, // Message ID: Unconnected pong
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x98, 0x76, 0x54, 0x32, // Data
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let pong = UnconnectedPongMessage::read_message(&mut reader).expect("Failed to read unconnected pong");

        // Assert
        assert_eq!(0x0123456789ABCDEF, pong.time);
        assert_eq!(0x8877665544332211, pong.guid);
        assert_eq!(vec![0x98, 0x76, 0x54, 0x32], pong.data);
    }

    #[test]
    fn read_unconnected_pong_empty_data() {
        // Arrange
        let buf = vec![
            0x1C, // Message ID: Unconnected pong
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            // Empty data
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let pong = UnconnectedPongMessage::read_message(&mut reader).expect("Failed to read unconnected pong");

        // Assert
        assert_eq!(0x0123456789ABCDEF, pong.time);
        assert_eq!(0x8877665544332211, pong.guid);
        assert_eq!(Vec::<u8>::new(), pong.data);
    }

    #[test]
    fn read_unconnected_pong_invalid_offline_message_id() {
        // Arrange
        let buf = vec![
            0x1C, // Message ID: Unconnected pong
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0xAA, 0xAA, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // INVALID Offline message ID
            0x98, 0x76, 0x54, 0x32, // Data
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let result = UnconnectedPongMessage::read_message(&mut reader);

        // Assert
        match result {
            Ok(_) => panic!("Message read even though offline message ID was incorrect"),
            Err(RakNetError::InvalidData) => {},
            _ => panic!("Invalid error reading message with invalid offline message ID"),
        }
    }    

    #[test]
    fn write_unconnected_pong() {
        // Arrange
        let pong = UnconnectedPongMessage {
            time: 0x0123456789ABCDEF,
            guid: 0x8877665544332211,
            data: vec![0x98, 0x76, 0x54, 0x32],
        };
        let mut buf = Vec::new();

        // Act
        pong.write_message(&mut buf).expect("Could not write pong message");

        // Assert
        assert_eq!(vec![
            0x1C, // Message ID: Unconnected pong
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x98, 0x76, 0x54, 0x32, // Data
        ],
        buf);
    }

    #[test]
    fn write_unconnected_pong_empty_data() {
        // Arrange
        let pong = UnconnectedPongMessage {
            time: 0x0123456789ABCDEF,
            guid: 0x8877665544332211,
            data: vec![],
        };
        let mut buf = Vec::new();

        // Act
        pong.write_message(&mut buf).expect("Could not write pong message with empty data");

        // Assert
        assert_eq!(vec![
            0x1C, // Message ID: Unconnected pong
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // Time: 0x0123456789ABCDEF
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            // Empty data
        ],
        buf);
    }

    #[test]
    fn read_open_connection_request_1() {
        // Arrange
        let buf = vec![
            0x05, // Message ID: Open Connection Request 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x12, // RakNet protocol version: 0x12
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Zero padding: 8 bytes
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let req1 = OpenConnectionRequest1Message::read_message(&mut reader).expect("Failed to read Open Connection Request 1");

        // Assert
        assert_eq!(0x12, req1.protocol_version);
        assert_eq!(8, req1.padding_length);
    }

    #[test]
    fn read_open_connection_request_1_invalid_offline_message_id() {
        // Arrange
        let buf = vec![
            0x05, // Message ID: Open Connection Request 1
            0xAA, 0xAA, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // INVALID Offline message ID
            0x12, // RakNet protocol version: 0x12
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Zero padding: 8 bytes
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let result = OpenConnectionRequest1Message::read_message(&mut reader);

        // Assert
        match result {
            Ok(_) => panic!("Message read even though offline message ID was incorrect"),
            Err(RakNetError::InvalidData) => {},
            _ => panic!("Invalid error reading message with invalid offline message ID"),
        }
    }    

    #[test]
    fn write_open_connection_request_1() {
        // Arrange
        let req1 = OpenConnectionRequest1Message {
            protocol_version: 0x34,
            padding_length: 10,
        };
        let mut buf = Vec::new();

        // Act
        req1.write_message(&mut buf).expect("Could not write Open Connection Request 1");

        // Assert
        assert_eq!(vec![
            0x05, // Message ID: Open Connection Request 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x34, // RakNet protocol version: 0x34
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Zero padding: 10 bytes
        ],
        buf);
    }

    #[test]
    fn read_incompatible_protocol_version() {
        // Arrange
        let buf = vec![
            0x19, // Message ID: Incompatible Protocol Version
            0x23, // Protocol version: 0x23
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let message = IncompatibleProtocolVersionMessage::read_message(&mut reader).expect("Failed to message");

        // Assert
        assert_eq!(0x23, message.protocol_version);
        assert_eq!(0x8877665544332211, message.guid);
    }

    #[test]
    fn read_incompatible_protocol_version_invalid_offline_message_id() {
        // Arrange
        let buf = vec![
            0x19, // Message ID: Incompatible Protocol Version
            0x23, // Protocol version: 0x23
            0xAA, 0xAA, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let result = IncompatibleProtocolVersionMessage::read_message(&mut reader);

        // Assert
        match result {
            Ok(_) => panic!("Message read even though offline message ID was incorrect"),
            Err(RakNetError::InvalidData) => {},
            _ => panic!("Invalid error reading message with invalid offline message ID"),
        }
    }    

    #[test]
    fn write_incompatible_protocol_version() {
        // Arrange
        let message = IncompatibleProtocolVersionMessage {
            protocol_version: 0x23,
            guid: 0x8877665544332211,
        };
        let mut buf = Vec::new();

        // Act
        message.write_message(&mut buf).expect("Could not write message");

        // Assert
        assert_eq!(vec![
            0x19, // Message ID: Incompatible Protocol Version
            0x23, // Protocol version: 0x23
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
        ],
        buf);
    }
}
