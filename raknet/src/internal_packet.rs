use std::time::Instant;

use crate::{
    constants::NUMBER_OF_ORDERING_CHANNELS,
    error::ReadError,
    number::{MessageNumber, OrderingChannelIndex, OrderingIndex, SequencingIndex},
    DataRead,
    Result
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Reliability {
    Unreliable,
    Reliable(MessageNumber),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Ordering {
    None,
    Ordered { ordering_index: OrderingIndex, ordering_channel_index: OrderingChannelIndex },
    Sequenced { sequencing_index: SequencingIndex, ordering_index: OrderingIndex, ordering_channel_index: OrderingChannelIndex },
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SplitPacketHeader {
    split_packet_count: u32,
    split_packet_id: u16,
    split_packet_index: u32,
}

impl SplitPacketHeader {
    pub fn read(reader: &mut impl DataRead) -> Result<Self> {
        let header = SplitPacketHeader {
            split_packet_count: reader.read_u32_be()?,
            split_packet_id: reader.read_u16_be()?,
            split_packet_index: reader.read_u32_be()?,
        };
        if header.split_packet_index >= header.split_packet_index {
            return Err(ReadError::InvalidHeader.into());
        }
        Ok(header)
    }

    pub fn split_packet_count(&self) -> u32 {
        self.split_packet_count
    }

    pub fn split_packet_id(&self) -> u16 {
        self.split_packet_id
    }

    pub fn split_packet_index(&self) -> u32 {
        self.split_packet_index
    }
}

#[derive(Debug)]
pub struct InternalPacket {
    creation_time: Instant,
    reliability: Reliability,
    ordering: Ordering,
    split_packet_header: Option<SplitPacketHeader>,
    payload: Box<[u8]>, 
}

impl InternalPacket {
    pub fn read(creation_time: Instant, reader: &mut impl DataRead) -> Result<Self> {        
        let flags = reader.read_u8()?;
        let payload_bit_length = reader.read_u16_be()?;
        let payload_byte_length = (payload_bit_length + 8 - 1) / 8;
        if payload_byte_length == 0 {
            return Err(ReadError::InvalidHeader.into());
        }
        let (reliability, ordering) = match (flags & 0b1110_0000) >> 5 {
            0 => (Reliability::Unreliable, Ordering::None),
            1 => {
                let sequencing_index = SequencingIndex::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                if ordering_channel_index >= NUMBER_OF_ORDERING_CHANNELS {
                    return Err(ReadError::InvalidHeader.into());
                }
                (Reliability::Unreliable, Ordering::Sequenced {
                    sequencing_index,
                    ordering_index,
                    ordering_channel_index,
                })
            },
            2 => (Reliability::Reliable(MessageNumber::from(reader.read_u24()?)), Ordering::None),
            3 => {
                let reliable_message_number = MessageNumber::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                if ordering_channel_index >= NUMBER_OF_ORDERING_CHANNELS {
                    return Err(ReadError::InvalidHeader.into());
                }
                (Reliability::Reliable(reliable_message_number), Ordering::Ordered {
                    ordering_index,
                    ordering_channel_index,
                })
            },
            4 => {
                let reliable_message_number = MessageNumber::from(reader.read_u24()?);
                let sequencing_index = SequencingIndex::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                if ordering_channel_index >= NUMBER_OF_ORDERING_CHANNELS {
                    return Err(ReadError::InvalidHeader.into());
                }
                (Reliability::Reliable(reliable_message_number),Ordering::Sequenced {
                    sequencing_index,
                    ordering_index,
                    ordering_channel_index,
                })
            },
            _ => return Err(ReadError::InvalidHeader.into()),
        };
        let has_split_packet = (flags & 0b0001_0000) != 0;
        let split_packet_header = if has_split_packet {
            Some(SplitPacketHeader::read(reader)?)
        } else {
            None
        };
        let payload = reader.read_bytes_to_boxed_slice(payload_byte_length as usize)?;
        Ok(InternalPacket {
            creation_time,
            reliability,
            ordering,
            split_packet_header,
            payload,
        })
    }

    pub fn reliability(&self) -> Reliability {
        self.reliability
    }

    pub fn ordering(&self) -> Ordering {
        self.ordering
    }

    pub fn split_packet_header(&self) -> Option<SplitPacketHeader> {
        self.split_packet_header
    }

    #[allow(dead_code)]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Box<[u8]> {
        self.payload
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use crate::reader::DataReader;
    use super::{InternalPacket, Ordering, Reliability};

    #[test]
    fn read_unreliable_packet() {
        // Arrange
        let buf = [
            0x00, // Bitflags: bit 7-5: reliability=0=Unreliable, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), Reliability::Unreliable));
        assert!(matches!(packet.ordering(), Ordering::None));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }
}