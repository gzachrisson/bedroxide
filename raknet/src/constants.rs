use std::time::Duration;

pub const OFFLINE_MESSAGE_ID: [u8; 16] = [0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78];

pub const RAKNET_PROTOCOL_VERSION: u8 = 10;

pub const UDP_HEADER_SIZE: u16 = 28;

pub const MAXIMUM_MTU_SIZE: u16 = 1492;

pub const NUMBER_OF_ORDERING_CHANNELS: u8 = 32;

pub const NUMBER_OF_PRIORITIES: usize = 4;

pub const TIME_BEFORE_SENDING_ACKS: Duration = Duration::from_millis(10);

pub const MAX_ACK_DATAGRAM_HEADER_SIZE: usize = 1 + 4; // Bitflags (u8) + AS (f32)

pub const MAX_NACK_DATAGRAM_HEADER_SIZE: usize = 1; // Bitflags (u8)

pub const MAX_NUMBER_OF_INTERNAL_IDS: usize = 10;