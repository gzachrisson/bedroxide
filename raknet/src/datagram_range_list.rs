use std::convert::TryFrom;

use crate::{
    datagram_range::DatagramRange, Result, WriteError, writer::DataWrite
};

#[derive(Debug)]
pub struct DatagramRangeList {
    ranges: Vec<DatagramRange>,
    bytes_used: usize,
}

impl DatagramRangeList {
    pub fn new() -> Self {
        DatagramRangeList {
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

    pub fn push(&mut self, range: DatagramRange) {
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
            Err(WriteError::TooManyRanges.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use crate::{datagram_range::DatagramRange, DatagramSequenceNumber};
    use super::DatagramRangeList;

    #[test]
    fn datagram_range_list_write_one_range_one_datagram() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(5u8)));
        let mut buf = Vec::new();

        // Act
        range_list.write(&mut buf).expect("Couldn't write ack");

        // Assert
        assert_eq!(buf, vec![
            0x00, 0x01, // Range count: 0x0001
            0x01, // Start equal to end? 0x01=yes
            0x05, 0x00, 0x00, // Datagram number: 0x000005 
        ]);
    }

    #[test]
    fn datagram_range_list_write_one_range_multiple_acks() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0xFFu8)));
        let mut buf = Vec::new();

        // Act
        range_list.write(&mut buf).expect("Couldn't write ack");

        // Assert
        assert_eq!(buf, vec![
            0x00, 0x01, // Range count: 0x0001
            0x00, // Start equal to end? 0x00=no
            0x00, 0x00, 0x00, // Start datagram number: 0x000000
            0xFF, 0x00, 0x00, // End datagram number: 0x0000FF
        ]);
    }    

    #[test]
    fn datagram_range_list_write_multiple_ranges() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));
        let mut buf = Vec::new();

        // Act
        range_list.write(&mut buf).expect("Couldn't write ack");

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
    fn datagram_range_list_bytes_used_one_range_one_ack() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(5u8)));

        // Act/Assert        
        assert_eq!(range_list.bytes_used(), 2 + (1 + 3));
    }   

    #[test]
    fn datagram_range_list_bytes_used_multiple_ranges() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));

        // Act/Assert        
        assert_eq!(range_list.bytes_used(), 2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3));
    }

    #[test]
    fn datagram_range_list_is_full_no_range() {
        // Arrange
        let range_list = DatagramRangeList::new();

        // Act/Assert        
        assert!(range_list.is_full(2));
        assert!(range_list.is_full(2 + (1 + 3 + 2)));

        assert!(!range_list.is_full(2 + (1 + 3 + 3)));
        assert!(!range_list.is_full(2 + (1 + 3 + 4)));
    }

    #[test]
    fn datagram_range_list_is_full_multiple_ranges() {
        // Arrange
        let mut range_list = DatagramRangeList::new();
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(0u8), DatagramSequenceNumber::from(0u8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::from(5u8), DatagramSequenceNumber::from(0xFFu8)));
        range_list.push(DatagramRange::new(DatagramSequenceNumber::try_from(0x123456).unwrap(), DatagramSequenceNumber::try_from(0x334455).unwrap()));

        // Act/Assert        
        assert!(range_list.is_full(2 + (1 + 3 + 3 )));
        assert!(range_list.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3)));
        assert!(range_list.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 2)));

        assert!(!range_list.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 3)));
        assert!(!range_list.is_full(2 + (1 + 3) + (1 + 3 + 3) + (1 + 3 + 3) + (1 + 3 + 4)));
    }
}