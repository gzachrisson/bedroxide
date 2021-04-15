use core::convert::TryFrom;

use super::RakNetError;

#[derive(Copy, Clone, Debug)]
pub enum MessageId {
    UnconnectedPing = 0x01,
    OpenConnectionRequest1 = 0x05,
    OpenConnectionReply1 = 0x06,
    OpenConnectionRequest2 = 0x07,
    IncompatibleProtocolVersion = 0x19,
    UnconnectedPong = 0x1c,
}

impl From<MessageId> for u8 {
    fn from(message_id: MessageId) -> u8 {
        message_id as u8
    }
}

impl TryFrom<u8> for MessageId {
    type Error = RakNetError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::UnconnectedPing),
            0x05 => Ok(Self::OpenConnectionRequest1),
            0x06 => Ok(Self::OpenConnectionReply1),
            0x07 => Ok(Self::OpenConnectionRequest2),
            0x19 => Ok(Self::IncompatibleProtocolVersion),
            0x1c => Ok(Self::UnconnectedPong),
            _ => Err(RakNetError::UnknownMessageId(value)),
        }
    }
}