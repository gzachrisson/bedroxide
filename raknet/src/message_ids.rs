use core::convert::TryFrom;

use crate::{Error, Result};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessageId {
    UnconnectedPing = 0x01,
    UnconnectedPingOpenConnections = 0x02,
    OpenConnectionRequest1 = 0x05,
    OpenConnectionReply1 = 0x06,
    OpenConnectionRequest2 = 0x07,
    OpenConnectionReply2 = 0x08,
    OutOfBandInternal = 0x0d,
    ConnectionAttemptFailed = 0x11,
    AlreadyConnected = 0x12,
    NoFreeIncomingConnections = 0x14,
    ConnectionBanned = 0x17,
    IncompatibleProtocolVersion = 0x19,
    IpRecentlyConnected = 0x1a,
    UnconnectedPong = 0x1c,
}

impl From<MessageId> for u8 {
    fn from(message_id: MessageId) -> u8 {
        message_id as u8
    }
}

impl TryFrom<u8> for MessageId {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::UnconnectedPing),
            0x02 => Ok(Self::UnconnectedPingOpenConnections),
            0x05 => Ok(Self::OpenConnectionRequest1),
            0x06 => Ok(Self::OpenConnectionReply1),
            0x07 => Ok(Self::OpenConnectionRequest2),
            0x08 => Ok(Self::OpenConnectionReply2),
            0x0D => Ok(Self::OutOfBandInternal),
            0x11 => Ok(Self::ConnectionAttemptFailed),
            0x12 => Ok(Self::AlreadyConnected),
            0x14 => Ok(Self::NoFreeIncomingConnections),
            0x17 => Ok(Self::ConnectionBanned),
            0x19 => Ok(Self::IncompatibleProtocolVersion),
            0x1a => Ok(Self::IpRecentlyConnected),
            0x1c => Ok(Self::UnconnectedPong),
            _ => Err(Error::UnknownMessageId(value)),
        }
    }
}