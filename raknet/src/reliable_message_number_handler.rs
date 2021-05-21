use std::collections::VecDeque;

use crate::number::MessageNumber;

pub struct ReliableMessageNumberHandler {
    holes: VecDeque<bool>,
    base_index: MessageNumber,
    next_outgoing_number: MessageNumber,
}

impl ReliableMessageNumberHandler {
    pub fn new() -> Self {
        ReliableMessageNumberHandler {
            holes: VecDeque::new(),
            base_index: MessageNumber::from_masked_u32(0),
            next_outgoing_number: MessageNumber::ZERO,
        }
    }

    pub fn get_and_increment_reliable_message_number(&mut self) -> MessageNumber {
        let number = self.next_outgoing_number;
        self.next_outgoing_number.wrapping_add(MessageNumber::ONE);
        number
    }

    /// Returns true if the message number has already been received
    /// and the packet is a duplicate that shall be discarded.
    pub fn should_discard_packet(&mut self, number: MessageNumber) -> bool {
        let offset = usize::from(number.wrapping_sub(self.base_index));
        if offset == 0 {
            // Got the number we were expecting
            if !self.holes.is_empty() {
                self.holes.pop_front();
            }
            self.base_index = self.base_index.wrapping_add(MessageNumber::ONE);
        } else if offset > MessageNumber::HALF_MAX.into() {
            // Duplicate packet
            return true;
        } else if offset < self.holes.len() {
            // Got an out of order number, lower than a previously received number
            if self.holes[offset] {
                // There was a hole in the number sequence, fill in the hole
                self.holes[offset] = false;
            } else {
                // Duplicate packet
                return true;
            }
        } else {
            // Got an out of order number, higher than the previously received numbers
            if offset > 1000000 {
                // Too big offset, would allocate too much memory
                return true;
            }
            // Fill with holes up to the received number
            while offset > self.holes.len() {
                self.holes.push_back(true);
            }
            // Add the received number
            self.holes.push_back(false);
        }
        // Pop all received numbers
        while self.holes.get(0) == Some(&false) {
            self.holes.pop_front();
            self.base_index = self.base_index.wrapping_add(MessageNumber::ONE); 
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::number::MessageNumber;
    use super::ReliableMessageNumberHandler;

    #[test]
    fn should_discard_packet_initial_state() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();

        // Act
        let should_discard = handler.should_discard_packet(MessageNumber::from_masked_u32(0));

        // Assert
        assert!(!should_discard);
    }

    #[test]
    fn should_discard_packet_ordered_numbers() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();

        // Act
        let should_discard1 = handler.should_discard_packet(MessageNumber::from_masked_u32(0));
        let should_discard2 = handler.should_discard_packet(MessageNumber::from_masked_u32(1));
        let should_discard3 = handler.should_discard_packet(MessageNumber::from_masked_u32(2));
        let should_discard4 = handler.should_discard_packet(MessageNumber::from_masked_u32(3));

        // Assert
        assert!(!should_discard1);
        assert!(!should_discard2);
        assert!(!should_discard3);
        assert!(!should_discard4);
    }

    #[test]
    fn should_discard_packet_unordered_numbers() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();

        // Act
        let should_discard1 = handler.should_discard_packet(MessageNumber::from_masked_u32(3));
        let should_discard2 = handler.should_discard_packet(MessageNumber::from_masked_u32(1));
        let should_discard3 = handler.should_discard_packet(MessageNumber::from_masked_u32(0));
        let should_discard4 = handler.should_discard_packet(MessageNumber::from_masked_u32(2));
        
        // Assert
        assert!(!should_discard1);
        assert!(!should_discard2);
        assert!(!should_discard3);
        assert!(!should_discard4);
    }

    #[test]
    fn should_discard_packet_duplicate_numbers() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();

        // Act
        let should_discard1 = handler.should_discard_packet(MessageNumber::from_masked_u32(0));
        let should_discard2 = handler.should_discard_packet(MessageNumber::from_masked_u32(1));
        let should_discard3 = handler.should_discard_packet(MessageNumber::from_masked_u32(0));
        let should_discard4 = handler.should_discard_packet(MessageNumber::from_masked_u32(1));
        let should_discard5 = handler.should_discard_packet(MessageNumber::from_masked_u32(2));
        
        // Assert
        assert!(!should_discard1);
        assert!(!should_discard2);
        assert!(should_discard3);
        assert!(should_discard4);
        assert!(!should_discard5);       
    }

    #[test]
    fn should_discard_packet_too_out_of_order() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();

        // Act
        let should_discard1 = handler.should_discard_packet(MessageNumber::from_masked_u32(1000000));
        let should_discard2 = handler.should_discard_packet(MessageNumber::from_masked_u32(1000001));
        let should_discard3 = handler.should_discard_packet(MessageNumber::HALF_MAX + MessageNumber::ONE);

        // Assert
        assert!(!should_discard1);
        assert!(should_discard2);
        assert!(should_discard3);
    }

    #[test]
    fn should_discard_packet_wrapping() {
        // Arrange
        let mut handler = ReliableMessageNumberHandler::new();
        let mut number = MessageNumber::ZERO;
        loop {
            assert!(!handler.should_discard_packet(number));
            if number < MessageNumber::MAX {
                number = number + MessageNumber::ONE;
            } else {
                break;
            }
        }

        // Act
        let should_discard = handler.should_discard_packet(MessageNumber::ZERO);

        // Assert
        assert!(!should_discard);
    }
}