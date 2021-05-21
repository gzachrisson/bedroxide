use std::time::Instant;

use crate::{
    error::{ReadError, WriteError},
    number::{MessageNumber, OrderingChannelIndex, OrderingIndex, SequencingIndex},
    reader::DataRead,
    Result,
    writer::DataWrite,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InternalReliability {
    Unreliable,
    Reliable(Option<MessageNumber>),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InternalOrdering {
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
    #[allow(dead_code)]
    pub fn new(split_packet_count: u32, split_packet_id: u16, split_packet_index: u32) -> Self {
        SplitPacketHeader {
            split_packet_count,
            split_packet_id,
            split_packet_index,
        }
    }

    pub fn read(reader: &mut impl DataRead) -> Result<Self> {
        let header = SplitPacketHeader {
            split_packet_count: reader.read_u32_be()?,
            split_packet_id: reader.read_u16_be()?,
            split_packet_index: reader.read_u32_be()?,
        };
        Ok(header)
    }

    #[allow(dead_code)]
    pub fn write(&self, writer: &mut impl DataWrite) -> Result<()> {
        writer.write_u32_be(self.split_packet_count)?;
        writer.write_u16_be(self.split_packet_id)?;
        writer.write_u32_be(self.split_packet_index)?;
        Ok(())
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
    reliability: InternalReliability,
    ordering: InternalOrdering,
    split_packet_header: Option<SplitPacketHeader>,
    receipt: Option<u32>,
    payload: Box<[u8]>, 
}

impl InternalPacket {
    #[allow(dead_code)]
    pub fn new(creation_time: Instant, reliability: InternalReliability, ordering: InternalOrdering, split_packet_header: Option<SplitPacketHeader>, receipt: Option<u32>, payload: Box<[u8]>) -> Self {
        InternalPacket {
            creation_time,
            reliability,
            ordering,
            split_packet_header,
            receipt,
            payload,
        }
    }

    pub fn read(creation_time: Instant, reader: &mut impl DataRead) -> Result<Self> { 
        let flags = reader.read_u8()?;
        let payload_bit_length = reader.read_u16_be()?;
        let payload_byte_length = (payload_bit_length + 8 - 1) / 8;
        if payload_byte_length == 0 {
            return Err(ReadError::InvalidHeader.into());
        }
        let (reliability, ordering) = match (flags & 0b1110_0000) >> 5 {
            0 => (InternalReliability::Unreliable, InternalOrdering::None),
            1 => {
                let sequencing_index = SequencingIndex::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                (InternalReliability::Unreliable, InternalOrdering::Sequenced {
                    sequencing_index,
                    ordering_index,
                    ordering_channel_index,
                })
            },
            2 => (InternalReliability::Reliable(Some(MessageNumber::from(reader.read_u24()?))), InternalOrdering::None),
            3 => {
                let reliable_message_number = MessageNumber::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                (InternalReliability::Reliable(Some(reliable_message_number)), InternalOrdering::Ordered {
                    ordering_index,
                    ordering_channel_index,
                })
            },
            4 => {
                let reliable_message_number = MessageNumber::from(reader.read_u24()?);
                let sequencing_index = SequencingIndex::from(reader.read_u24()?);
                let ordering_index = OrderingIndex::from(reader.read_u24()?);
                let ordering_channel_index = OrderingChannelIndex::from(reader.read_u8()?);
                (InternalReliability::Reliable(Some(reliable_message_number)),InternalOrdering::Sequenced {
                    sequencing_index,
                    ordering_index,
                    ordering_channel_index,
                })
            },
            _ => return Err(ReadError::InvalidHeader.into()),
        };
        let has_split_packet = (flags & 0b000_1_0000) != 0;
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
            receipt: None,
            payload,
        })
    }

    #[allow(dead_code)]
    pub fn write(&self, writer: &mut impl DataWrite) -> Result<()> {
        let mut flags: u8 = match (self.reliability, self.ordering) {
            (InternalReliability::Unreliable, InternalOrdering::None) => 0 << 5,
            (InternalReliability::Unreliable, InternalOrdering::Sequenced {sequencing_index: _, ordering_index: _, ordering_channel_index: _}) => 1 << 5,
            (InternalReliability::Reliable(_), InternalOrdering::None) => 2 << 5,
            (InternalReliability::Reliable(_), InternalOrdering::Ordered { ordering_index: _, ordering_channel_index: _ }) => 3 << 5,
            (InternalReliability::Reliable(_), InternalOrdering::Sequenced {sequencing_index: _, ordering_index: _, ordering_channel_index: _}) => 4 << 5,
            _ => return Err(WriteError::InvalidHeader.into()),
        };
        if let Some(_) = self.split_packet_header {
            flags = flags | 0b000_1_0000;
        }
        writer.write_u8(flags)?;

        if self.payload.len() > (u16::MAX / 8) as usize {
            return Err(WriteError::PayloadTooLarge.into());
        }
        writer.write_u16_be((self.payload.len() * 8) as u16)?;

        if let InternalReliability::Reliable(reliable_message_number) = self.reliability {
            if let Some(reliable_message_number) = reliable_message_number {
                writer.write_u24(reliable_message_number)?;
            } else {
                return Err(WriteError::InvalidHeader.into());
            }
        }
        match self.ordering {
            InternalOrdering::Sequenced {sequencing_index, ordering_index, ordering_channel_index } => {
                writer.write_u24(sequencing_index)?;
                writer.write_u24(ordering_index)?;
                writer.write_u8(ordering_channel_index)?;
            },
            InternalOrdering::Ordered { ordering_index, ordering_channel_index } => {
                writer.write_u24(ordering_index)?;
                writer.write_u8(ordering_channel_index)?;
            },
            _ => {},
        }

        if let Some(split_packet_header) = self.split_packet_header {
            split_packet_header.write(writer)?;
        }

        writer.write_bytes(&self.payload)?;
        Ok(())
    }

    pub fn reliability(&self) -> InternalReliability {
        self.reliability
    }

    pub fn set_reliability(&mut self, reliability: InternalReliability) {
        self.reliability = reliability;
    }

    pub fn ordering(&self) -> InternalOrdering {
        self.ordering
    }

    pub fn split_packet_header(&self) -> Option<SplitPacketHeader> {
        self.split_packet_header
    }

    pub fn get_size_in_bytes(&self) -> u16 {
        self.get_header_size_in_bytes() + self.payload.len() as u16
    }

    fn get_header_size_in_bytes(&self) -> u16 {
        // Bitflags (u8) + Data bit length (u16)
        let mut header_size = 1 + 2;
        if let InternalReliability::Reliable(_) = self.reliability {
            // Reliable message number (u24)
            header_size = header_size + 3;
        }
        header_size = header_size + match self.ordering {
            InternalOrdering::None => 0,
            InternalOrdering::Ordered { ordering_index: _, ordering_channel_index: _ } => 3 + 1,
            InternalOrdering::Sequenced { sequencing_index: _, ordering_index: _, ordering_channel_index: _ } => 3 + 3 + 1,
        };
        if let Some(_) = self.split_packet_header {
            // Split packet count (u32) + split packet ID (u16) + split packet index (u32)
            header_size = header_size + 4 + 2 + 4;
        }
        header_size
    }

    #[allow(dead_code)]
    pub fn receipt(&self) -> Option<u32> {
        self.receipt
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
    use std::{convert::TryFrom, time::Instant};
    use crate::{number::{MessageNumber, OrderingIndex, SequencingIndex}, reader::DataReader};
    use super::{InternalPacket, InternalOrdering, InternalReliability, SplitPacketHeader};

    #[test]
    fn read_unreliable_packet() {
        // Arrange
        let buf = [
            0b000_0_0000, // Bitflags: bit 7-5: reliability=0=Unreliable, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Unreliable));
        assert!(matches!(packet.ordering(), InternalOrdering::None));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_unreliable_split_packet() {
        // Arrange
        let buf = [
            0b000_1_0000, // Bitflags: bit 7-5: reliability=0=Unreliable, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Unreliable));
        assert!(matches!(packet.ordering(), InternalOrdering::None));
        assert!(matches!(packet.split_packet_header(), Some(header)
            if header.split_packet_count() == 0x11223344 &&
                header.split_packet_id() == 0x1357 &&
                header.split_packet_index() == 0x01234567
        ));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_unreliable_sequenced_packet() {
        // Arrange
        let buf = [
            0b001_0_0000, // Bitflags: bit 7-5: reliability=1=Unreliable Sequenced, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Sequencing index: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Unreliable));
        assert!(matches!(packet.ordering(), InternalOrdering::Sequenced {
            sequencing_index,
            ordering_index,
            ordering_channel_index: 0x05
        } if sequencing_index == SequencingIndex::try_from(0x123456).unwrap() &&
            ordering_index == OrderingIndex::try_from(0x112233).unwrap()
        ));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }    

    #[test]
    fn read_unreliable_sequenced_split_packet() {
        // Arrange
        let buf = [
            0b001_1_0000, // Bitflags: bit 7-5: reliability=1=Unreliable Sequenced, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Sequencing index: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Unreliable));
        assert!(matches!(packet.ordering(), InternalOrdering::Sequenced {
            sequencing_index,
            ordering_index,
            ordering_channel_index: 0x05
        } if sequencing_index == SequencingIndex::try_from(0x123456).unwrap() &&
            ordering_index == OrderingIndex::try_from(0x112233).unwrap()
        ));
        assert!(matches!(packet.split_packet_header(), Some(header)
            if header.split_packet_count() == 0x11223344 &&
                header.split_packet_id() == 0x1357 &&
                header.split_packet_index() == 0x01234567
        ));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }    

    #[test]
    fn read_reliable_packet() {
        // Arrange
        let buf = [
            0b010_0_0000, // Bitflags: bit 7-5: reliability=2=Reliable, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::None));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_reliable_split_packet() {
        // Arrange
        let buf = [
            0b010_1_0000, // Bitflags: bit 7-5: reliability=2=Reliable, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::None));
        assert!(matches!(packet.split_packet_header(), Some(header)
            if header.split_packet_count() == 0x11223344 &&
                header.split_packet_id() == 0x1357 &&
                header.split_packet_index() == 0x01234567
        ));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_reliable_ordered_packet() {
        // Arrange
        let buf = [
            0b011_0_0000, // Bitflags: bit 7-5: reliability=3=Reliable Ordered, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::Ordered {
            ordering_index,
            ordering_channel_index: 0x05
        } if ordering_index == OrderingIndex::try_from(0x112233).unwrap()));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }    

    #[test]
    fn read_reliable_ordered_split_packet() {
        // Arrange
        let buf = [
            0b011_1_0000, // Bitflags: bit 7-5: reliability=3=Reliable Ordered, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::Ordered {
            ordering_index,
            ordering_channel_index: 0x05
        } if ordering_index == OrderingIndex::try_from(0x112233).unwrap()));
        assert!(matches!(packet.split_packet_header(), Some(header)
            if header.split_packet_count() == 0x11223344 &&
                header.split_packet_id() == 0x1357 &&
                header.split_packet_index() == 0x01234567
        ));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_reliable_sequenced_packet() {
        // Arrange
        let buf = [
            0b100_0_0000, // Bitflags: bit 7-5: reliability=4=Reliable Sequenced, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x30, 0x20, 0x10, // Sequencing index: 0x102030
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::Sequenced {
            sequencing_index,
            ordering_index,
            ordering_channel_index: 0x05
        } if sequencing_index == SequencingIndex::try_from(0x102030).unwrap() &&
            ordering_index == OrderingIndex::try_from(0x112233).unwrap()
        ));
        assert!(matches!(packet.split_packet_header(), None));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn read_reliable_sequenced_split_packet() {
        // Arrange
        let buf = [
            0b100_1_0000, // Bitflags: bit 7-5: reliability=4=Reliable Sequenced, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x30, 0x20, 0x10, // Sequencing index: 0x102030
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ];
        let mut reader = DataReader::new(&buf);

        // Act
        let packet = InternalPacket::read(Instant::now(), &mut reader).expect("Failed to read packet");

        // Assert
        assert!(matches!(packet.reliability(), InternalReliability::Reliable(Some(message_number)) if message_number == MessageNumber::try_from(0x123456).unwrap()));
        assert!(matches!(packet.ordering(), InternalOrdering::Sequenced {
            sequencing_index,
            ordering_index,
            ordering_channel_index: 0x05
        } if sequencing_index == SequencingIndex::try_from(0x102030).unwrap() &&
            ordering_index == OrderingIndex::try_from(0x112233).unwrap()
        ));
        assert!(matches!(packet.split_packet_header(), Some(header)
            if header.split_packet_count() == 0x11223344 &&
                header.split_packet_id() == 0x1357 &&
                header.split_packet_index() == 0x01234567
        ));
        assert_eq!(packet.payload(), &[0x12, 0x34]);
    }

    #[test]
    fn write_unreliable_packet() {
        // Arrange
        let packet = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b000_0_0000, // Bitflags: bit 7-5: reliability=0=Unreliable, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_unreliable_split_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Unreliable,
            InternalOrdering::None,
            Some(SplitPacketHeader::new(0x11223344, 0x1357, 0x01234567)),
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b000_1_0000, // Bitflags: bit 7-5: reliability=0=Unreliable, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }    

    #[test]
    fn write_unreliable_sequenced_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Unreliable,
            InternalOrdering::Sequenced {
                sequencing_index: SequencingIndex::from_masked_u32(0x123456),
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            None,
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b001_0_0000, // Bitflags: bit 7-5: reliability=1=Unreliable Sequenced, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Sequencing index: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_unreliable_sequenced_split_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Unreliable,
            InternalOrdering::Sequenced {
                sequencing_index: SequencingIndex::from_masked_u32(0x123456),
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            Some(SplitPacketHeader::new(0x11223344, 0x1357, 0x01234567)),
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b001_1_0000, // Bitflags: bit 7-5: reliability=1=Unreliable Sequenced, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Sequencing index: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }    

    #[test]
    fn write_reliable_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::None,
            None,
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b010_0_0000, // Bitflags: bit 7-5: reliability=2=Reliable, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_reliable_split_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::None,
            Some(SplitPacketHeader::new(0x11223344, 0x1357, 0x01234567)),
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b010_1_0000, // Bitflags: bit 7-5: reliability=2=Reliable, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_reliable_ordered_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::Ordered {
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            None,
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b011_0_0000, // Bitflags: bit 7-5: reliability=3=Reliable Ordered, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
   }    

    #[test]
    fn write_reliable_ordered_split_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::Ordered {
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            Some(SplitPacketHeader::new(0x11223344, 0x1357, 0x01234567)),
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec![
            0b011_1_0000, // Bitflags: bit 7-5: reliability=3=Reliable Ordered, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_reliable_sequenced_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::Sequenced {
                sequencing_index: SequencingIndex::from_masked_u32(0x102030),
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            None,
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec! [
            0b100_0_0000, // Bitflags: bit 7-5: reliability=4=Reliable Sequenced, bit 4: has_split_packet=0=false
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x30, 0x20, 0x10, // Sequencing index: 0x102030
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }

    #[test]
    fn write_reliable_sequenced_split_packet() {
        // Arrange
        let packet = InternalPacket::new(
            Instant::now(),
            InternalReliability::Reliable(Some(MessageNumber::from_masked_u32(0x123456))),
            InternalOrdering::Sequenced {
                sequencing_index: SequencingIndex::from_masked_u32(0x102030),
                ordering_index: OrderingIndex::from_masked_u32(0x112233),
                ordering_channel_index: 0x05
            },
            Some(SplitPacketHeader::new(0x11223344, 0x1357, 0x01234567)),
            vec![0x12, 0x34].into_boxed_slice());
        let mut buf = Vec::new();

        // Act
        packet.write(&mut buf).expect("Could not write packet");

        // Assert
        assert_eq!(buf, vec! [
            0b100_1_0000, // Bitflags: bit 7-5: reliability=4=Reliable Sequenced, bit 4: has_split_packet=1=true
            0x00, 0x10, // Data bit length: 0x0010=16 bits=2 bytes
            0x56, 0x34, 0x12, // Reliable message number: 0x123456
            0x30, 0x20, 0x10, // Sequencing index: 0x102030
            0x33, 0x22, 0x11, // Ordering index: 0x112233
            0x05, // Ordering channel: 5
            0x11, 0x22, 0x33, 0x44, // Split packet count: 0x11223344
            0x13, 0x57, // Split packet ID: 0x1357
            0x01, 0x23, 0x45, 0x67, // Split packet index: 0x01234567 
            0x12, 0x34, // Data [0x12, 0x34]
        ]);
    }
}