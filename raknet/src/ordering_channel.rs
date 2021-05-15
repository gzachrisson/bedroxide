use std::{cmp::{Ord, Ordering}, collections::BinaryHeap};

use crate::number::{OrderingIndex, SequencingIndex};

struct PacketWithWeight {
    pub weight: u64,
    pub sequencing_index: Option<SequencingIndex>,
    pub ordering_index: OrderingIndex,
    pub payload: Box<[u8]>,
}

impl Ord for PacketWithWeight {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering
        other.weight.cmp(&self.weight)
    }
}

impl PartialOrd for PacketWithWeight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PacketWithWeight {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}

impl Eq for PacketWithWeight {}

pub struct OrderingChannel {
    ordering_index_offset: OrderingIndex,
    expected_ordering_index: OrderingIndex,
    expected_sequencing_index: SequencingIndex,
    packets: BinaryHeap<PacketWithWeight>,
}

impl OrderingChannel {
    pub fn new() -> Self {
        OrderingChannel {
            ordering_index_offset: OrderingIndex::ZERO,
            expected_ordering_index: OrderingIndex::ZERO,
            expected_sequencing_index: SequencingIndex::ZERO,
            packets: BinaryHeap::new(),
        }
    }

    pub fn process_incoming(&mut self, sequencing_index: Option<SequencingIndex>, ordering_index: OrderingIndex, payload: Box<[u8]>) -> Option<Box<[u8]>> {
        if ordering_index == self.expected_ordering_index {
            if let Some(sequencing_index) = sequencing_index {
                if sequencing_index.wrapping_less_than(self.expected_sequencing_index) {
                    // Older sequencing index, drop packet
                    None
                } else {
                    // Got a sequenced packet with sequencing index greater than or equal to the expected, return packet
                    self.expected_sequencing_index = sequencing_index.wrapping_add(SequencingIndex::ONE);
                    Some(payload)
                }
            } else {
                // Got an ordered packet with the expected ordering index, return packet
                self.expected_sequencing_index = SequencingIndex::ZERO;
                self.expected_ordering_index = self.expected_ordering_index.wrapping_add(OrderingIndex::ONE);
                Some(payload)
            }
        } else if ordering_index.wrapping_less_than(self.expected_ordering_index) {
            // Older ordering index, drop packet
            None
        } else {
            // Higher ordering index than expected, buffer packet

            // Keep hole count low
            if self.packets.is_empty() {
                self.ordering_index_offset = self.expected_ordering_index;
            }
            let ordered_hole_count = ordering_index.wrapping_sub(self.ordering_index_offset);
            let mut weight = u64::from(ordered_hole_count) << 32;
            if let Some(sequencing_index) = sequencing_index {
                weight = weight + u64::from(sequencing_index);
            } else {
                weight = weight + 0xFFFFFFFF;
            }
            self.packets.push(PacketWithWeight {weight, sequencing_index, ordering_index, payload});
            None
        }
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            expected_ordering_index: &mut self.expected_ordering_index,
            expected_sequencing_index: &mut self.expected_sequencing_index,
            packets: & mut self.packets,
        }
    }
}

pub struct IterMut<'a> {
    expected_ordering_index: &'a mut OrderingIndex,
    expected_sequencing_index: &'a mut SequencingIndex,
    packets: &'a mut BinaryHeap<PacketWithWeight>,    
}

impl<'a> Iterator for IterMut<'a> {
    type Item = Box<[u8]>;

    fn next(&mut self) -> Option<Box<[u8]>> {
        if let Some(packet) = self.packets.peek() {
            if packet.ordering_index == *self.expected_ordering_index {
                if let Some(packet) = self.packets.pop() {
                    if let Some(sequencing_index) = packet.sequencing_index {
                        *self.expected_sequencing_index = sequencing_index.wrapping_add(SequencingIndex::ONE);
                    } else {
                        *self.expected_ordering_index = self.expected_ordering_index.wrapping_add(OrderingIndex::ONE);
                        *self.expected_sequencing_index = SequencingIndex::ZERO;
                    }
                    Some(packet.payload)
                } else {
                    // Should not happen since peek succeeded
                    None
                }
            } else {
                // Incorrect ordering index
                None
            }
        } else {
            // Heap is empty
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::number::{OrderingIndex, SequencingIndex};
    use super::OrderingChannel;

    #[test]
    fn initial_state() {
        // Arrange/Act
        let mut channel = OrderingChannel::new();

        // Assert
        let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
        assert!(packets.is_empty());
    }

    #[test]
    fn process_incoming_sequenced_packet_expected_ordering_index() {
        // Arrange
        let mut channel = OrderingChannel::new();

        // Act
        let packet = channel.process_incoming(Some(SequencingIndex::ONE), OrderingIndex::ZERO, vec![1, 2, 3].into_boxed_slice());

        // Assert
        assert_eq!(packet, Some(vec![1, 2, 3].into_boxed_slice()));
        let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
        assert!(packets.is_empty());
    }

    #[test]
    fn process_incoming_sequenced_packet_old_ordering_index() {
        // Arrange
        let mut channel = OrderingChannel::new();

        // Act
        let packet1 = channel.process_incoming(None, OrderingIndex::ZERO, vec![9, 9, 9].into_boxed_slice());
        let packet2 = channel.process_incoming(Some(SequencingIndex::ONE), OrderingIndex::ZERO, vec![1, 2, 3].into_boxed_slice());

        // Assert
        assert_eq!(packet1, Some(vec![9, 9, 9].into_boxed_slice()));
        assert_eq!(packet2, None);
        let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
        assert!(packets.is_empty());
    }


    #[test]
    fn process_incoming_sequenced_packet_old_sequencing_index() {
        // Arrange
        let mut channel = OrderingChannel::new();

        // Act
        let packet1 = channel.process_incoming(Some(SequencingIndex::ONE), OrderingIndex::ZERO, vec![1, 2, 3].into_boxed_slice());
        let packet2 = channel.process_incoming(Some(SequencingIndex::ZERO), OrderingIndex::ZERO, vec![3, 4, 5].into_boxed_slice());

        // Assert
        assert_eq!(packet1, Some(vec![1, 2, 3].into_boxed_slice()));
        assert_eq!(packet2, None);
        let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
        assert!(packets.is_empty());
    }

    #[test]
    fn process_incoming_sequenced_packet_higher_ordering_index_than_expected() {
        // Arrange
        let mut channel = OrderingChannel::new();

        // Act
        let packet1 = channel.process_incoming(Some(SequencingIndex::ONE), OrderingIndex::ONE, vec![1, 2, 3].into_boxed_slice());
        let packets1: Vec<Box<[u8]>> = channel.iter_mut().collect();
        let packet2 = channel.process_incoming(None, OrderingIndex::ZERO, vec![9, 9, 9].into_boxed_slice());
        let packets2: Vec<Box<[u8]>> = channel.iter_mut().collect();
        
        // Assert
        assert_eq!(packet1, None);
        assert!(packets1.is_empty());
        assert_eq!(packet2, Some(vec![9, 9, 9].into_boxed_slice()));
        assert_eq!(packets2, vec![vec![1, 2, 3].into_boxed_slice()]);
    }

    #[test]
    fn process_incoming_ordered_packet_higher_ordering_index_than_expected() {
        // Arrange
        let mut channel = OrderingChannel::new();

        // Act
        let packet1 = channel.process_incoming(None, OrderingIndex::ONE, vec![1, 2, 3].into_boxed_slice());
        let packets1: Vec<Box<[u8]>> = channel.iter_mut().collect();
        let packet2 = channel.process_incoming(None, OrderingIndex::ZERO, vec![9, 9, 9].into_boxed_slice());
        let packets2: Vec<Box<[u8]>> = channel.iter_mut().collect();
        
        // Assert
        assert_eq!(packet1, None);
        assert!(packets1.is_empty());
        assert_eq!(packet2, Some(vec![9, 9, 9].into_boxed_slice()));
        assert_eq!(packets2, vec![vec![1, 2, 3].into_boxed_slice()]);
    }

    #[test]
    fn process_incoming_sequenced_packet_wrapping_sequencing_index() {
        // Arrange
        let mut channel = OrderingChannel::new();
        let mut sequencing_index = SequencingIndex::ZERO;
        loop {
            let packet = channel.process_incoming(Some(sequencing_index), OrderingIndex::ZERO, vec![1, 2, 3].into_boxed_slice());
            assert_eq!(packet, Some(vec![1, 2, 3].into_boxed_slice()));
            let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
            assert!(packets.is_empty());
            if sequencing_index < SequencingIndex::MAX - SequencingIndex::from_masked_u32(500) {
                sequencing_index = sequencing_index + SequencingIndex::from_masked_u32(500);
            } else {
                break;
            }
        }        

        // Act
        let packet = channel.process_incoming(Some(SequencingIndex::ZERO), OrderingIndex::ZERO, vec![1, 2, 3].into_boxed_slice());
        let packets: Vec<Box<[u8]>> = channel.iter_mut().collect();
        
        // Assert
        assert_eq!(packet, Some(vec![1, 2, 3].into_boxed_slice()));
        assert!(packets.is_empty());
    }      
}