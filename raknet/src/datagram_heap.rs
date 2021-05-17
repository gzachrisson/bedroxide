use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{datagram_range::DatagramRange, number::DatagramSequenceNumber};

pub struct DatagramHeap {
    datagrams: BinaryHeap<Reverse<DatagramSequenceNumber>>,
}

impl DatagramHeap {
    pub fn new() -> Self {
        DatagramHeap {
            datagrams: BinaryHeap::new(),
        }
    }

    pub fn push(&mut self, number: DatagramSequenceNumber) {
        self.datagrams.push(Reverse(number));
    }

    pub fn is_empty(&self) -> bool {
        self.datagrams.is_empty()
    }

    pub fn pop_range(&mut self) -> Option<DatagramRange> {
        if let Some(Reverse(first_number)) = self.datagrams.pop() {
            let mut range = DatagramRange::new(first_number, first_number);
            while let Some(Reverse(number)) = self.datagrams.peek() {
                if range.push(*number) {
                    self.datagrams.pop();
                } else {
                    break;
                }
            }
            Some(range)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{datagram_range::DatagramRange, number::DatagramSequenceNumber};
    use super::DatagramHeap;

    #[test]
    fn datagram_heap_is_empty_initial_state_empty() {
        // Arrange
        let heap = DatagramHeap::new();

        // Act/Assert
        assert!(heap.is_empty());
    }

    #[test]
    fn datagram_heap_is_empty_empty() {
        // Arrange
        let mut heap = DatagramHeap::new();
        heap.push(DatagramSequenceNumber::from(5u8));
        heap.pop_range();

        // Act/Assert
        assert!(heap.is_empty());
    }

    #[test]
    fn datagram_heap_is_empty_not_empty() {
        // Arrange
        let mut heap = DatagramHeap::new();
        heap.push(DatagramSequenceNumber::from(5u8));

        // Act/Assert
        assert!(!heap.is_empty());
    }

    #[test]
    fn datagram_heap_pop_range_empty() {
        // Arrange
        let mut heap = DatagramHeap::new();

        // Act/Assert
        assert_eq!(heap.pop_range(), None);
    }

    #[test]
    fn datagram_heap_pop_range_one_range_start_end_same() {
        // Arrange
        let mut heap = DatagramHeap::new();
        heap.push(DatagramSequenceNumber::from(1u8));

        // Act
        let range = heap.pop_range();
        let empty = heap.pop_range();

        //Assert
        assert_eq!(range, Some(DatagramRange::new(1u8.into(), 1u8.into())));
        assert_eq!(empty, None);
    }

    #[test]
    fn datagram_heap_pop_range_one_range_start_end_different() {
        // Arrange
        let mut heap = DatagramHeap::new();
        heap.push(DatagramSequenceNumber::from(1u8));
        heap.push(DatagramSequenceNumber::from(2u8));
        heap.push(DatagramSequenceNumber::from(3u8));

        // Act
        let range = heap.pop_range();
        let empty = heap.pop_range();

        //Assert
        assert_eq!(range, Some(DatagramRange::new(1u8.into(), 3u8.into())));
        assert_eq!(empty, None);
    }

    #[test]
    fn datagram_heap_pop_range_multiple_ranges() {
        // Arrange
        let mut heap = DatagramHeap::new();
        heap.push(DatagramSequenceNumber::from(1u8));

        heap.push(DatagramSequenceNumber::from(5u8));
        heap.push(DatagramSequenceNumber::from(6u8));
        heap.push(DatagramSequenceNumber::from(7u8));

        heap.push(DatagramSequenceNumber::from(10u8));
        heap.push(DatagramSequenceNumber::from(11u8));

        heap.push(DatagramSequenceNumber::from(20u8));

        // Act
        let range1 = heap.pop_range();
        let range2 = heap.pop_range();
        let range3 = heap.pop_range();
        let range4 = heap.pop_range();
        let empty = heap.pop_range();

        //Assert
        assert_eq!(range1, Some(DatagramRange::new(1u8.into(), 1u8.into())));
        assert_eq!(range2, Some(DatagramRange::new(5u8.into(), 7u8.into())));
        assert_eq!(range3, Some(DatagramRange::new(10u8.into(), 11u8.into())));
        assert_eq!(range4, Some(DatagramRange::new(20u8.into(), 20u8.into())));
        assert_eq!(empty, None);
    }
}