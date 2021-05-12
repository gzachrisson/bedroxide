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
        if Self::less_than(number, self.expected_next_number) {
            return;
        }

        let mut expected_number = self.expected_next_number;
        // Limit NACKs to 1000 for the datagram and use timeout resend for the rest
        // if this datagram really was valid.
        let mut nack_count = 0;
        while expected_number != number && nack_count < 1000 {
            self.nacks.push(expected_number);
            expected_number = expected_number.wrapping_add(DatagramSequenceNumber::from(1u8));
            nack_count = nack_count + 1;
        }

        self.expected_next_number = number.wrapping_add(DatagramSequenceNumber::from(1u8));
    }

    fn less_than(a: DatagramSequenceNumber, b: DatagramSequenceNumber) -> bool {        
        b != a && b.wrapping_sub(a) < DatagramSequenceNumber::HALF
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
    use super::OutgoingNacks;

    #[test]
    fn outgoing_nacks_is_empty_initial_state_empty() {
        // Arrange
        let acks = OutgoingNacks::new();

        // Act/Assert
        assert!(acks.is_empty());
    }

    // TODO: Add more tests of is_empty()
    // TODO: Add tests of handle_datagram()
    // TODO: Add tests of pop_range()    
}