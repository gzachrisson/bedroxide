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