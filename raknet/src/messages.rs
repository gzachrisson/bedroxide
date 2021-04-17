use std::net::SocketAddr;

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
        reader.read_u8_and_compare(MessageId::UnconnectedPing.into())?;
        let time = reader.read_u64_be()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let client_guid = reader.read_u64_be()?;
        Ok(UnconnectedPingMessage { time, client_guid })
    }
}

impl RakNetMessageWrite for UnconnectedPingMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::UnconnectedPing.into())?;
        writer.write_u64_be(self.time)?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_u64_be(self.client_guid)?;
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
        reader.read_u8_and_compare(MessageId::UnconnectedPong.into())?;
        let time = reader.read_u64_be()?;
        let guid = reader.read_u64_be()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let mut data = Vec::new();
        reader.read_bytes_to_end(&mut data)?;
        Ok(UnconnectedPongMessage { time, guid, data })
    }
}

impl RakNetMessageWrite for UnconnectedPongMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::UnconnectedPong.into())?;
        writer.write_u64_be(self.time)?;
        writer.write_u64_be(self.guid)?;
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
        reader.read_u8_and_compare(MessageId::OpenConnectionRequest1.into())?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let protocol_version = reader.read_u8()?;
        let padding_length = reader.read_zero_padding()?;
        Ok(OpenConnectionRequest1Message { protocol_version, padding_length })
    }
}

impl RakNetMessageWrite for OpenConnectionRequest1Message {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::OpenConnectionRequest1.into())?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_u8(self.protocol_version)?;
        writer.write_zero_padding(self.padding_length)?;
        Ok(())      
    }
}

pub struct OpenConnectionReply1Message {
    pub guid: u64,
    pub cookie_and_public_key: Option<(u32, [u8;64])>,
    pub mtu: u16,
}

impl RakNetMessageRead for OpenConnectionReply1Message {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_u8_and_compare(MessageId::OpenConnectionReply1.into())?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let guid = reader.read_u64_be()?;
        let use_security = reader.read_u8()?;
        let cookie_and_public_key = if use_security == 0x01 {
            let mut public_key = [0u8; 64];
            let cookie = reader.read_u32_be()?;
            reader.read_bytes(&mut public_key)?;
            Some((cookie, public_key))
        } else {
            None
        };
        let mtu = reader.read_u16_be()?;

        Ok(OpenConnectionReply1Message {
            guid,
            cookie_and_public_key,
            mtu,
        })
    }
}

impl RakNetMessageWrite for OpenConnectionReply1Message {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::OpenConnectionReply1.into())?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_u64_be(self.guid)?;
        if let Some((cookie, public_key)) = self.cookie_and_public_key {
            writer.write_u8(0x01)?; // Using security = 0x01
            writer.write_u32_be(cookie)?;
            writer.write_bytes(&public_key)?;
        } else {
            writer.write_u8(0x00)?; // Not using security = 0x00
        }
        writer.write_u16_be(self.mtu)?;
        Ok(())      
    }
}

pub struct OpenConnectionRequest2Message {
    pub cookie_and_challenge: Option<(u32, Option<[u8; 64]>)>,
    pub binding_address: SocketAddr,
    pub mtu: u16,
    pub guid: u64,
}

impl RakNetMessageRead for OpenConnectionRequest2Message {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_u8_and_compare(MessageId::OpenConnectionRequest2.into())?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let binding_address = reader.read_socket_addr()?;
        let mtu = reader.read_u16_be()?;
        let guid = reader.read_u64_be()?;
        Ok(OpenConnectionRequest2Message {
            cookie_and_challenge: None,
            binding_address,
            mtu,
            guid,
        })
    }

    fn read_message_with_security(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_u8_and_compare(MessageId::OpenConnectionRequest2.into())?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let cookie = reader.read_u32_be()?;
        let client_wrote_challenge = reader.read_u8()?;
        let challenge = if client_wrote_challenge != 0x00 { 
            let mut challenge = [0u8; 64];
            reader.read_bytes(&mut challenge)?;
            Some(challenge)
        } else {
            None
        };
        let binding_address = reader.read_socket_addr()?;
        let mtu = reader.read_u16_be()?;
        let guid = reader.read_u64_be()?;
        Ok(OpenConnectionRequest2Message {
            cookie_and_challenge: Some((cookie, challenge)),
            binding_address,
            mtu,
            guid,
        })
    }    
}

impl RakNetMessageWrite for OpenConnectionRequest2Message {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::OpenConnectionRequest2.into())?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        if let Some((cookie, challenge)) = &self.cookie_and_challenge {
            writer.write_u32_be(*cookie)?;
            if let Some(challenge) = challenge {
                writer.write_u8(0x01)?; // Client wrote challenge: true
                writer.write_bytes(challenge)?;
            } else {
                writer.write_u8(0x00)?; // Client wrote challenge: false
            }
        }
        writer.write_socket_addr(&self.binding_address)?;
        writer.write_u16_be(self.mtu)?;
        writer.write_u64_be(self.guid)?;
        Ok(())      
    }
}

pub struct IncompatibleProtocolVersionMessage {
    pub protocol_version: u8,
    pub guid: u64,
}

impl RakNetMessageRead for IncompatibleProtocolVersionMessage {
    fn read_message(reader: &mut dyn RakNetRead) -> Result<Self, RakNetError> {
        reader.read_u8_and_compare(MessageId::IncompatibleProtocolVersion.into())?;
        let protocol_version = reader.read_u8()?;
        reader.read_bytes_and_compare(&OFFLINE_MESSAGE_ID)?;
        let guid = reader.read_u64_be()?;
        Ok(IncompatibleProtocolVersionMessage { protocol_version, guid })
    }
}

impl RakNetMessageWrite for IncompatibleProtocolVersionMessage {
    fn write_message(&self, writer: &mut dyn RakNetWrite) -> Result<(), RakNetError> {
        writer.write_u8(MessageId::IncompatibleProtocolVersion.into())?;
        writer.write_u8(self.protocol_version)?;
        writer.write_bytes(&OFFLINE_MESSAGE_ID)?;
        writer.write_u64_be(self.guid)?;
        Ok(())      
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Cursor,
        net::SocketAddr,
    };

    use crate::{
        error::RakNetError,
        messages::{
            UnconnectedPingMessage,
            UnconnectedPongMessage,
            OpenConnectionRequest1Message,
            OpenConnectionReply1Message,
            OpenConnectionRequest2Message,
            IncompatibleProtocolVersionMessage},
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
    fn read_open_connection_reply_1_no_security() {
        // Arrange
        let buf = vec![
            0x06, // Message ID: Open Connection Reply 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, // Use security: false = 0x00
            0x01, 0x23, // MTU: 0x0123
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let reply1 = OpenConnectionReply1Message::read_message(&mut reader).expect("Failed to read message");

        // Assert
        assert_eq!(0x8877665544332211, reply1.guid);
        assert_eq!(None, reply1.cookie_and_public_key);
        assert_eq!(0x0123, reply1.mtu);
    }

    #[test]
    fn read_open_connection_reply_1_with_security() {
        // Arrange
        let buf = vec![
            0x06, // Message ID: Open Connection Reply 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x01, // Use security: true = 0x01
            0x11, 0x22, 0x33, 0x44, // Cookie (4 bytes)
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, // Public key (4 bytes)
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4,
            0x01, 0x23, // MTU size: 0x0123
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let reply1 = OpenConnectionReply1Message::read_message(&mut reader).expect("Failed to read message");

        // Assert
        assert_eq!(0x8877665544332211, reply1.guid);
        assert_eq!(Some((0x11223344, [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0,
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4,            
        ])),
        reply1.cookie_and_public_key);
        assert_eq!(0x0123, reply1.mtu);
    }

    #[test]
    fn write_open_connection_reply_1_no_security() {
        // Arrange
        let reply1 = OpenConnectionReply1Message {
            guid: 0x8877665544332211,
            cookie_and_public_key: None,
            mtu: 0x0123,
        };
        let mut buf = Vec::new();

        // Act
        reply1.write_message(&mut buf).expect("Could not write message");

        // Assert
        assert_eq!(vec![
            0x06, // Message ID: Open Connection Reply 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x00, // Use security: false = 0x00
            0x01, 0x23, // MTU size: 0x0123
        ],
        buf);
    }
    
    #[test]
    fn write_open_connection_reply_1_with_security() {
        // Arrange
        let reply1 = OpenConnectionReply1Message {
            guid: 0x8877665544332211,
            cookie_and_public_key: Some((0x11223344,[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0,
                1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4,   
            ])),
            mtu: 0x0123,
        };
        let mut buf = Vec::new();

        // Act
        reply1.write_message(&mut buf).expect("Could not write message");

        // Assert
        assert_eq!(vec![
            0x06, // Message ID: Open Connection Reply 1
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, // Guid: 0x8877665544332211
            0x01, // Use security: true = 0x01
            0x11, 0x22, 0x33, 0x44, // Cookie (4 bytes)
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, // Public key (4 bytes)
            1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4,
            0x01, 0x23, // MTU size: 0x0123
        ],
        buf);
    }

    #[test]
    fn read_open_connection_request_2_no_security() {
        // Arrange
        let buf = vec![
            0x07, // Message ID: Open Connection Request 2
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x04, !192, !168, !1, !248, 0x12, 0x34, // Binding address IPv4: 192.168.1.248, port: 0x1234
            0x01, 0x23, // MTU: 0x123
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, // GUID: 0x123456789ABCDEF0
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let req2 = OpenConnectionRequest2Message::read_message(&mut reader).expect("Failed to read message");

        // Assert
        assert_eq!(SocketAddr::from(([192, 168, 1, 248], 0x1234)), req2.binding_address);
        assert_eq!(0x123, req2.mtu);
        assert_eq!(0x123456789ABCDEF0, req2.guid);
    }

    #[test]
    fn read_open_connection_request_2_with_security_no_challenge() {
        // Arrange
        let buf = vec![
            0x07, // Message ID: Open Connection Request 2
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x12, 0x34, 0x56, 0x78, // Cookie: 0x12345678
            0x00, // Client wrote challenge: 0x00 = false
            0x04, !192, !168, !1, !248, 0x12, 0x34, // Binding address IPv4: 192.168.1.248, port: 0x1234
            0x01, 0x23, // MTU: 0x123
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, // GUID: 0x123456789ABCDEF0
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let req2 = OpenConnectionRequest2Message::read_message_with_security(&mut reader).expect("Failed to read message");

        // Assert
        assert_eq!(Some((0x12345678u32, None)), req2.cookie_and_challenge);
        assert_eq!(SocketAddr::from(([192, 168, 1, 248], 0x1234)), req2.binding_address);
        assert_eq!(0x123, req2.mtu);
        assert_eq!(0x123456789ABCDEF0, req2.guid);
    }

    #[test]
    fn read_open_connection_request_2_with_security_with_challenge() {
        // Arrange
        let buf = vec![
            0x07, // Message ID: Open Connection Request 2
            0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78, // Offline message ID
            0x12, 0x34, 0x56, 0x78, // Cookie: 0x12345678
            0x01, // Client wrote challenge: 0x01 = true
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, // Challenge: 64 bytes
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
            0x04, !192, !168, !1, !248, 0x12, 0x34, // Binding address IPv4: 192.168.1.248, port: 0x1234
            0x01, 0x23, // MTU: 0x123
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, // GUID: 0x123456789ABCDEF0
        ];
        let mut reader = Cursor::new(buf);

        // Act
        let req2 = OpenConnectionRequest2Message::read_message_with_security(&mut reader).expect("Failed to read message");

        // Assert
        assert_eq!(Some((0x12345678u32, Some([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, // Challenge: 64 bytes
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
            ]))),
            req2.cookie_and_challenge);
        assert_eq!(SocketAddr::from(([192, 168, 1, 248], 0x1234)), req2.binding_address);
        assert_eq!(0x123, req2.mtu);
        assert_eq!(0x123456789ABCDEF0, req2.guid);
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
