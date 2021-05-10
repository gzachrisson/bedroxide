use std::{cmp::Reverse, collections::BinaryHeap, convert::TryFrom, time::Instant};

use crate::{constants::TIME_BEFORE_SENDING_ACKS, number::DatagramSequenceNumber, Result, WriteError, writer::DataWrite};

pub struct OutgoingAcknowledgements {
    acks: BinaryHeap<Reverse<DatagramSequenceNumber>>,
    oldest_ack_time: Option<Instant>,
}

impl OutgoingAcknowledgements {
    pub fn new() -> Self {
        OutgoingAcknowledgements {
            acks: BinaryHeap::new(),
            oldest_ack_time: None,
        }
    }

    pub fn insert(&mut self, number: DatagramSequenceNumber, time: Instant) {
        if self.acks.is_empty() {
            self.oldest_ack_time = Some(time);
        }
        self.acks.push(Reverse(number));
    }

    pub fn is_empty(&self) -> bool {
        self.acks.is_empty()
    }

    pub fn pop_range(&mut self) -> Option<AcknowledgementRange> {
        if let Some(Reverse(first_number)) = self.acks.pop() {
            let mut range = AcknowledgementRange::new(first_number, first_number);
            while let Some(Reverse(number)) = self.acks.peek() {
                if range.push(*number) {
                    self.acks.pop();
                } else {
                    break;
                }
            }
            if self.acks.is_empty() {
                self.oldest_ack_time = None;
            }
            Some(range)
        } else {
            None
        }
    }

    pub fn should_send_acks(&self, current_time: Instant) -> bool {
        if let Some(oldest_ack_time) = self.oldest_ack_time {
            current_time.saturating_duration_since(oldest_ack_time) > TIME_BEFORE_SENDING_ACKS
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct AcknowledgementRange {
    start: DatagramSequenceNumber,
    end: DatagramSequenceNumber,
}

impl AcknowledgementRange {
    pub fn new(start: DatagramSequenceNumber, end: DatagramSequenceNumber) -> Self {
        AcknowledgementRange {
            start,
            end,
        }
    }

    /// Pushes the number to the end of the range if the number
    /// immediately follows the end of the range.
    /// Returns true if the number could be pushed and false otherwise.
    pub fn push(&mut self, number: DatagramSequenceNumber) -> bool {
        if let Some(next_number) = self.next_number() {
            if number == next_number {
                self.end = next_number;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn start(&self) -> DatagramSequenceNumber {
        self.start
    }

    pub fn end(&self) -> DatagramSequenceNumber {
        self.end
    }

    fn next_number(&self) -> Option<DatagramSequenceNumber> {
        if self.end != DatagramSequenceNumber::MAX {
            Some(self.end + DatagramSequenceNumber::from(1u8))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Acknowledgement {
    ranges: Vec<AcknowledgementRange>,
    bytes_used: usize,
}

impl Acknowledgement {
    pub fn new() -> Self {
        Acknowledgement {
            ranges: Vec::new(),
            bytes_used: std::mem::size_of::<u16>(), // Range count (u16)
        }
    }

    pub fn is_full(&self, max_datagram_payload: usize) -> bool {
        // Check if we can add one more range of the longest kind:
        // "start not equal to end" (u8) + "start" (u24) + "end" (u24)
        self.bytes_used + 1 + 3 + 3 > max_datagram_payload
    }
    
    pub fn bytes_used(&self) -> usize {
        self.bytes_used
    }

    pub fn push(&mut self, range: AcknowledgementRange) {
        self.bytes_used = if range.start() == range.end() {
            // "start equal to end" (u8) + "start" (u24)
            self.bytes_used + 1 + 3
        } else {
            // "start not equal to end" (u8) + "start" (u24) + "end" (u24)
            self.bytes_used + 1 + 3 + 3
        };
        self.ranges.push(range);
    }

    pub fn write(&self, writer: &mut dyn DataWrite) -> Result<()> {
        if let Ok(number_of_ranges) = u16::try_from(self.ranges.len()) {
            writer.write_u16_be(number_of_ranges)?;
            for range in self.ranges.iter() {
                if range.start() == range.end() {
                    writer.write_u8(0x01)?; // Start equal to end = 1
                    writer.write_u24(range.start())?;
                } else {
                    writer.write_u8(0x00)?; // Start not equal to end = 0
                    writer.write_u24(range.start())?;
                    writer.write_u24(range.end())?;
                }
            }
            Ok(())
        } else {
            Err(WriteError::TooManyAckRanges.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use crate::DatagramSequenceNumber;
    use super::{Acknowledgement, AcknowledgementRange};

    #[test]
    fn acknowledgement_write_one_range_one_ack() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(5u8)));
        let mut buf = Vec::new();

        // Act
        ack.write(&mut buf).expect("Couldn't write ack");

        // Assert
        assert_eq!(buf, vec![
            0x00, 0x01, // Range count: 0x0001
            0x01, // Start equal to end? 0x01=yes
            0x05, 0x00, 0x00, // Datagram number: 0x000005 
        ]);
    }

    #[test]
    fn acknowledgement_write_one_range_multiple_acks() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0xFFu8)));
        let mut buf = Vec::new();

        // Act
        ack.write(&mut buf).expect("Couldn't write ack");

        // Assert
        assert_eq!(buf, vec![
            0x00, 0x01, // Range count: 0x0001
            0x00, // Start equal to end? 0x00=no
            0x00, 0x00, 0x00, // Start datagram number: 0x000000
            0xFF, 0x00, 0x00, // End datagram number: 0x0000FF
        ]);
    }    

    #[test]
    fn acknowledgement_write_multiple_ranges() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));
        let mut buf = Vec::new();

        // Act
        ack.write(&mut buf).expect("Couldn't write ack");

        // Assert
        assert_eq!(buf, vec![
            0x00, 0x03, // Range count: 0x0003
            0x01, // Start equal to end? 0x01=yes
            0x00, 0x00, 0x00, // Datagram number: 0x000000
            0x00, // Start equal to end? 0x00=no
            0x05, 0x00, 0x00, // Start datagram number: 0x000005
            0xFF, 0x00, 0x00, // End datagram number: 0x0000FF
            0x00, // Start equal to end? 0x00=no
            0x56, 0x34, 0x12, // Start datagram number: 0x123456
            0x55, 0x44, 0x33, // End datagram number: 0x334455
        ]);
    }    

    #[test]
    fn acknowledgement_bytes_used_one_range_one_ack() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(5u8)));

        // Act/Assert        
        assert_eq!(ack.bytes_used(), 2 + (1 + 3));
    }   

    #[test]
    fn acknowledgement_bytes_used_multiple_ranges() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));

        // Act/Assert        
        assert_eq!(ack.bytes_used(), 2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3));
    }

    #[test]
    fn acknowledgement_is_full_no_range() {
        // Arrange
        let ack = Acknowledgement::new();

        // Act/Assert        
        assert!(ack.is_full(2));
        assert!(ack.is_full(2 + (1 + 3 + 2)));

        assert!(!ack.is_full(2 + (1 + 3 + 3)));
        assert!(!ack.is_full(2 + (1 + 3 + 4)));
    }

    #[test]
    fn acknowledgement_is_full_multiple_ranges() {
        // Arrange
        let mut ack = Acknowledgement::new();
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        ack.push(AcknowledgementRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));

        // Act/Assert        
        assert!(ack.is_full(2 + (1 + 3 + 3 )));
        assert!(ack.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3)));
        assert!(ack.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 2)));

        assert!(!ack.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 3)));
        assert!(!ack.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 4)));
    }

    #[test]
    fn acknowledgement_range_initial_values() {
        // Arrange
        let range = AcknowledgementRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(255u8));

        // Act/Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 255u8.into());
    }

    #[test]
    fn acknowledgement_push_can_push() {
        // Arrange
        let mut range = AcknowledgementRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(200u8));

        // Act/Assert
        assert!(range.push(201u8.into()));
        assert!(range.push(202u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 202u8.into());
    }

    #[test]
    fn acknowledgement_push_out_of_sequence() {
        // Arrange
        let mut range = AcknowledgementRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(200u8));

        // Act/Assert
        assert!(!range.push(199u8.into()));
        assert!(!range.push(202u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 200u8.into());
    }

    #[test]
    fn acknowledgement_push_end_of_sequence() {
        // Arrange
        let mut range = AcknowledgementRange::new(7u8.into(), DatagramSequenceNumber::try_from(0xFFFFFEu32).unwrap());

        // Act/Assert
        assert!(range.push(DatagramSequenceNumber::try_from(0xFFFFFFu32).unwrap()));
        assert!(!range.push(0u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), DatagramSequenceNumber::try_from(0xFFFFFFu32).unwrap());
    }
}