use crate::number::DatagramSequenceNumber;

#[derive(Debug, PartialEq)]
pub struct DatagramRange {
    start: DatagramSequenceNumber,
    end: DatagramSequenceNumber,
}

impl DatagramRange {
    pub fn new(start: DatagramSequenceNumber, end: DatagramSequenceNumber) -> Self {
        DatagramRange {
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

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use crate::number::DatagramSequenceNumber;
    use super::DatagramRange;

    #[test]
    fn datagram_range_initial_values() {
        // Arrange
        let range = DatagramRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(255u8));

        // Act/Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 255u8.into());
    }

    #[test]
    fn datagram_range_push_can_push() {
        // Arrange
        let mut range = DatagramRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(200u8));

        // Act/Assert
        assert!(range.push(201u8.into()));
        assert!(range.push(202u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 202u8.into());
    }

    #[test]
    fn datagram_range_push_out_of_sequence() {
        // Arrange
        let mut range = DatagramRange::new(DatagramSequenceNumber::from(7u8), DatagramSequenceNumber::from(200u8));

        // Act/Assert
        assert!(!range.push(199u8.into()));
        assert!(!range.push(202u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), 200u8.into());
    }

    #[test]
    fn datagram_range_push_end_of_sequence() {
        // Arrange
        let mut range = DatagramRange::new(7u8.into(), DatagramSequenceNumber::try_from(0xFFFFFEu32).unwrap());

        // Act/Assert
        assert!(range.push(DatagramSequenceNumber::try_from(0xFFFFFFu32).unwrap()));
        assert!(!range.push(0u8.into()));
        
        //Assert        
        assert_eq!(range.start(), 7u8.into());
        assert_eq!(range.end(), DatagramSequenceNumber::try_from(0xFFFFFFu32).unwrap());
    }
}