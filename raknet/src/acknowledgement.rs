use std::time::Instant;

use crate::{
    constants::TIME_BEFORE_SENDING_ACKS,
    datagram_heap::DatagramHeap,
    datagram_range::DatagramRange,
    number::DatagramSequenceNumber,
};

pub struct OutgoingAcknowledgements {
    acks: DatagramHeap,
    oldest_ack_time: Option<Instant>,
}

impl OutgoingAcknowledgements {
    pub fn new() -> Self {
        OutgoingAcknowledgements {
            acks: DatagramHeap::new(),
            oldest_ack_time: None,
        }
    }

    pub fn insert(&mut self, number: DatagramSequenceNumber, time: Instant) {
        if self.acks.is_empty() {
            self.oldest_ack_time = Some(time);
        }
        self.acks.push(number);
    }

    pub fn is_empty(&self) -> bool {
        self.acks.is_empty()
    }

    pub fn pop_range(&mut self) -> Option<DatagramRange> {
        if let Some(range) = self.acks.pop_range() {
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use crate::{constants::TIME_BEFORE_SENDING_ACKS, DatagramSequenceNumber};
    use super::{DatagramRange, OutgoingAcknowledgements};

    #[test]
    fn outgoing_acks_is_empty_initial_state_empty() {
        // Arrange
        let acks = OutgoingAcknowledgements::new();

        // Act/Assert
        assert!(acks.is_empty());
    }

    #[test]
    fn outgoing_acks_is_empty_empty() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        acks.insert(DatagramSequenceNumber::from(5u8), Instant::now());
        acks.pop_range();

        // Act/Assert
        assert!(acks.is_empty());
    }

    #[test]
    fn outgoing_acks_is_empty_not_empty() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        acks.insert(DatagramSequenceNumber::from(5u8), Instant::now());

        // Act/Assert
        assert!(!acks.is_empty());
    }

    #[test]
    fn outgoing_acks_pop_range_empty() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();

        // Act/Assert
        assert_eq!(acks.pop_range(), None);
    }

    #[test]
    fn outgoing_acks_pop_range_one_range_start_end_same() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        acks.insert(DatagramSequenceNumber::from(1u8), Instant::now());

        // Act
        let range = acks.pop_range();
        let empty = acks.pop_range();

        //Assert
        assert_eq!(range, Some(DatagramRange::new(1u8.into(), 1u8.into())));
        assert_eq!(empty, None);
    }

    #[test]
    fn outgoing_acks_pop_range_one_range_start_end_different() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        acks.insert(DatagramSequenceNumber::from(1u8), Instant::now());
        acks.insert(DatagramSequenceNumber::from(2u8), Instant::now());
        acks.insert(DatagramSequenceNumber::from(3u8), Instant::now());

        // Act
        let range = acks.pop_range();
        let empty = acks.pop_range();

        //Assert
        assert_eq!(range, Some(DatagramRange::new(1u8.into(), 3u8.into())));
        assert_eq!(empty, None);
    }

    #[test]
    fn outgoing_acks_pop_range_multiple_ranges() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        acks.insert(DatagramSequenceNumber::from(1u8), Instant::now());

        acks.insert(DatagramSequenceNumber::from(5u8), Instant::now());
        acks.insert(DatagramSequenceNumber::from(6u8), Instant::now());
        acks.insert(DatagramSequenceNumber::from(7u8), Instant::now());

        acks.insert(DatagramSequenceNumber::from(10u8), Instant::now());
        acks.insert(DatagramSequenceNumber::from(11u8), Instant::now());

        acks.insert(DatagramSequenceNumber::from(20u8), Instant::now());

        // Act
        let range1 = acks.pop_range();
        let range2 = acks.pop_range();
        let range3 = acks.pop_range();
        let range4 = acks.pop_range();
        let empty = acks.pop_range();

        //Assert
        assert_eq!(range1, Some(DatagramRange::new(1u8.into(), 1u8.into())));
        assert_eq!(range2, Some(DatagramRange::new(5u8.into(), 7u8.into())));
        assert_eq!(range3, Some(DatagramRange::new(10u8.into(), 11u8.into())));
        assert_eq!(range4, Some(DatagramRange::new(20u8.into(), 20u8.into())));
        assert_eq!(empty, None);
    }

    #[test]
    fn outgoing_acks_should_send_acks_initial_state() {
        // Arrange
        let acks = OutgoingAcknowledgements::new();

        // Act/Assert
        assert!(!acks.should_send_acks(Instant::now()));
    }

    #[test]
    fn outgoing_acks_should_send_acks_one_number() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        let time = Instant::now();
        acks.insert(DatagramSequenceNumber::from(1u8), time);

        // Act/Assert
        assert!(!acks.should_send_acks(time + (TIME_BEFORE_SENDING_ACKS - Duration::from_millis(1))));
        assert!(!acks.should_send_acks(time + TIME_BEFORE_SENDING_ACKS));
        assert!(acks.should_send_acks(time + (TIME_BEFORE_SENDING_ACKS + Duration::from_millis(1))));
    }

    #[test]
    fn outgoing_acks_should_send_acks_multiple_numbers() {
        // Arrange
        let mut acks = OutgoingAcknowledgements::new();
        let time = Instant::now();
        acks.insert(DatagramSequenceNumber::from(1u8), time);
        acks.insert(DatagramSequenceNumber::from(2u8), time + Duration::from_millis(100));
        acks.insert(DatagramSequenceNumber::from(10u8), time + Duration::from_millis(200));
        acks.insert(DatagramSequenceNumber::from(11u8), time + Duration::from_millis(300));

        // Act/Assert
        assert!(!acks.should_send_acks(time + (TIME_BEFORE_SENDING_ACKS - Duration::from_millis(1))));
        assert!(!acks.should_send_acks(time + TIME_BEFORE_SENDING_ACKS));
        assert!(acks.should_send_acks(time + (TIME_BEFORE_SENDING_ACKS + Duration::from_millis(1))));
    }    
}