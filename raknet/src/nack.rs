use crate::{
    datagram_heap::DatagramHeap,
    datagram_range::DatagramRange,
    number::DatagramSequenceNumber,
};

pub struct OutgoingNacks {
    nacks: DatagramHeap,
    expected_next_number: DatagramSequenceNumber,
}

impl OutgoingNacks {
    pub fn new() -> Self {
        OutgoingNacks {
            nacks: DatagramHeap::new(),
            expected_next_number: DatagramSequenceNumber::from(0u8),
        }
    }

    pub fn handle_datagram(&mut self, number: DatagramSequenceNumber) {
        if number.wrapping_less_than(self.expected_next_number) {
            // Duplicate datagram
            return;
        }

        let mut expected_number = self.expected_next_number;
        // Limit NACKs to 1000 for the datagram and use timeout resend for the rest
        // if this datagram really was valid.
        let mut nack_count = 0;
        while expected_number != number && nack_count < 1000 {
            self.nacks.push(expected_number);
            expected_number = expected_number.wrapping_add(DatagramSequenceNumber::ONE);
            nack_count = nack_count + 1;
        }

        self.expected_next_number = number.wrapping_add(DatagramSequenceNumber::ONE);
    }

    pub fn is_empty(&self) -> bool {
        self.nacks.is_empty()
    }

    pub fn pop_range(&mut self) -> Option<DatagramRange> {
        self.nacks.pop_range()
    }
}

#[cfg(test)]
mod tests {
    use crate::{datagram_range::DatagramRange, DatagramSequenceNumber};
    use super::OutgoingNacks;

    #[test]
    fn outgoing_nacks_is_empty_initial_state_empty() {
        // Arrange
        let nacks = OutgoingNacks::new();

        // Act/Assert
        assert!(nacks.is_empty());
    }

    #[test]
    fn outgoing_nacks_is_empty_not_empty() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));

        // Act/Assert
        assert!(!nacks.is_empty());
    }

    #[test]
    fn outgoing_nacks_is_empty_is_empty() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));
        nacks.pop_range();

        // Act/Assert
        assert!(nacks.is_empty());
    }

    #[test]
    fn outgoing_nacks_handle_datagram_no_missing_number() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(0));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(2));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(3));
        
        // Assert
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_missing_numbers() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(2));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(4));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(8));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(9));
        
        // Assert
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(0u8.into(), 0u8.into())));
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(3u8.into(), 3u8.into())));
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(5u8.into(), 7u8.into())));
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_more_than_1000_missing_numbers() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(0));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1500));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(3000));
        
        // Assert
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(1u16.into(), 1000u16.into())));
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(1501u16.into(), 2500u16.into())));
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_number_less_than_expected() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(0));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(2));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(1));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(3));
        
        // Assert
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_same_number_twice() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(0));
        nacks.handle_datagram(DatagramSequenceNumber::from_masked_u32(0));
        
        // Assert
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_number_wrapping_less_than_expected() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::HALF_MAX + 1u8.into());
        
        // Assert
        assert_eq!(nacks.pop_range(), None);
    }

    #[test]
    fn outgoing_nacks_handle_datagram_number_wrapping_greater_than_expected() {
        // Arrange
        let mut nacks = OutgoingNacks::new();
        
        // Act
        nacks.handle_datagram(DatagramSequenceNumber::HALF_MAX);
        
        // Assert
        assert_eq!(nacks.pop_range(), Some(DatagramRange::new(0u16.into(), 999u16.into())));
        assert_eq!(nacks.pop_range(), None);
    }
}