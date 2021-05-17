use core::convert::TryFrom;

use crate::{Error, Result};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessageId {
    ConnectedPing = 0x00,
    UnconnectedPing = 0x01,
    UnconnectedPingOpenConnections = 0x02,
    ConnectedPong = 0x03,
    DetectLostConnections = 0x04,
    OpenConnectionRequest1 = 0x05,
    OpenConnectionReply1 = 0x06,
    OpenConnectionRequest2 = 0x07,
    OpenConnectionReply2 = 0x08,
    ConnectionRequest = 0x09,
    // RemoteSystemRequiresPublicKey = 0x0a,
    // OurSystemRequiresSecurity = 0x0b,
    // PublicKeyMismatch = 0x0c
    OutOfBandInternal = 0x0d,
    // SndReceiptAcked = 0x0e,
    // SndReceiptLoss = 0x0f,
    ConnectionRequestAccepted = 0x10,
    ConnectionAttemptFailed = 0x11,
    AlreadyConnected = 0x12,
    NewIncomingConnection = 0x13,
    NoFreeIncomingConnections = 0x14,
    DisconnectionNotification = 0x15,
    ConnectionLost = 0x16,
    ConnectionBanned = 0x17,
    InvalidPassword = 0x18,
    IncompatibleProtocolVersion = 0x19,
    IpRecentlyConnected = 0x1a,
    // Timestamp = 0x1b,
    UnconnectedPong = 0x1c,
    // AdvertiseSystem = 0x1d,
    // DownloadProgress = 0x1e,
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
            0x00 => Ok(Self::ConnectedPing),
            0x01 => Ok(Self::UnconnectedPing),
            0x02 => Ok(Self::UnconnectedPingOpenConnections),
            0x03 => Ok(Self::ConnectedPong),
            0x04 => Ok(Self::DetectLostConnections),
            0x05 => Ok(Self::OpenConnectionRequest1),
            0x06 => Ok(Self::OpenConnectionReply1),
            0x07 => Ok(Self::OpenConnectionRequest2),
            0x08 => Ok(Self::OpenConnectionReply2),
            0x09 => Ok(Self::ConnectionRequest),
            // 0x0a => Ok(Self::RemoteSystemRequiresPublicKey),
            // 0x0b => Ok(Self::OurSystemRequiresSecurity),
            // 0x0c => Ok(Self::PublicKeyMismatch),
            0x0D => Ok(Self::OutOfBandInternal),
            // 0x0e => Ok(Self::SndReceiptAcked),
            // 0x0f => Ok(Self::SndReceiptLoss),
            0x10 => Ok(Self::ConnectionRequestAccepted),
            0x11 => Ok(Self::ConnectionAttemptFailed),
            0x12 => Ok(Self::AlreadyConnected),
            0x13 => Ok(Self::NewIncomingConnection),
            0x14 => Ok(Self::NoFreeIncomingConnections),
            0x15 => Ok(Self::DisconnectionNotification),
            0x16 => Ok(Self::ConnectionLost),
            0x17 => Ok(Self::ConnectionBanned),
            0x18 => Ok(Self::InvalidPassword),
            0x19 => Ok(Self::IncompatibleProtocolVersion),
            0x1a => Ok(Self::IpRecentlyConnected),
            // 0x1b => Ok(Self::Timestamp),
            0x1c => Ok(Self::UnconnectedPong),
            // 0x1d => Ok(Self::AdvertiseSystem),
            // 0x1e => Ok(Self::DownloadProgress),
            _ => Err(Error::UnknownMessageId(value)),
        }
    }
}