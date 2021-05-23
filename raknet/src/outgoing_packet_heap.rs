use std::{cmp::{Ord, Ordering}, collections::BinaryHeap};

use crate::{constants::NUMBER_OF_PRIORITIES, internal_packet::InternalPacket, packet::Priority};

type PriorityLevel = u64;
type HeapWeight = u64;

#[derive(Debug)]
struct HeapItem {
    weight: HeapWeight,
    priority_level: PriorityLevel,
    packet: InternalPacket,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering to make it a min-heap
        other.weight.cmp(&self.weight)
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}

impl Eq for HeapItem {}

#[derive(Debug)]
pub struct OutgoingPacketHeap {
    packets: BinaryHeap<HeapItem>,
    next_weights: [HeapWeight; NUMBER_OF_PRIORITIES],
}

impl OutgoingPacketHeap {
    pub fn new() -> Self {
        OutgoingPacketHeap {      
            packets: BinaryHeap::new(),
            next_weights: Self::get_initial_heap_weights(),
        }
    }

    pub fn push(&mut self, priority: Priority, packet: InternalPacket) {
        let weight = self.get_next_weight(priority);
        self.packets.push(HeapItem { weight, priority_level: priority as PriorityLevel, packet });
    }

    #[allow(dead_code)]
    pub fn pop(&mut self) -> Option<InternalPacket> {
        if let Some(item) = self.packets.pop() {
            Some(item.packet)
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&InternalPacket> {
        if let Some(item) = self.packets.peek() {
            Some(&item.packet)
        } else {
            None
        }
    }

    fn get_next_weight(&mut self, priority: Priority) -> HeapWeight {
        let priority_level = priority as u64;
        let mut next_weight = self.next_weights[priority_level as usize];
        if let Some(item) = self.packets.peek() {
            let peek_priority_level = item.priority_level;
            let peek_weight = item.weight;
            let min = peek_weight - (1 << peek_priority_level) * peek_priority_level + peek_priority_level;
            if next_weight < min {
                next_weight = min + (1 << priority_level) * priority_level + priority_level;
            }
            self.next_weights[priority_level as usize] = next_weight + (1 << priority_level) * (priority_level + 1) + priority_level;
        } else {
            self.next_weights = Self::get_initial_heap_weights();
        }
        next_weight
    }

    const fn get_initial_heap_weight(priority_level: u64) -> HeapWeight {
        (1 << priority_level) * priority_level + priority_level
    }

    const fn get_initial_heap_weights() -> [HeapWeight; NUMBER_OF_PRIORITIES] {
        [
            Self::get_initial_heap_weight(0),
            Self::get_initial_heap_weight(1),
            Self::get_initial_heap_weight(2),
            Self::get_initial_heap_weight(3),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use crate::{internal_packet::{InternalOrdering, InternalPacket, InternalReliability}, packet::Priority};
    use super::OutgoingPacketHeap;

    #[test]
    fn push_low_then_medium_priority_packets() {
        // Arrange
        let mut heap = OutgoingPacketHeap::new();
        let packet1 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());

        // Act
        heap.push(Priority::Low, packet1);
        heap.push(Priority::Medium, packet2);

        // Assert
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[2]));
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[1]));
        assert!(matches!(heap.pop(), None));
    }

    #[test]
    fn push_medium_then_low_priority_packets() {
        // Arrange
        let mut heap = OutgoingPacketHeap::new();
        let packet1 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());

        // Act
        heap.push(Priority::Medium, packet1);
        heap.push(Priority::Low, packet2);

        // Assert
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[1]));
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[2]));
        assert!(matches!(heap.pop(), None));
    }

    #[test]
    fn push_low_then_highest_priority_packets() {
        // Arrange
        let mut heap = OutgoingPacketHeap::new();
        let packet1 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());

        // Act
        heap.push(Priority::Low, packet1);
        heap.push(Priority::Highest, packet2);

        // Assert
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[2]));
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[1]));
        assert!(matches!(heap.pop(), None));
    }

    #[test]
    fn push_high_then_highest_priority_packets() {
        // Arrange
        let mut heap = OutgoingPacketHeap::new();
        let packet1 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());

        // Act
        heap.push(Priority::High, packet1);
        heap.push(Priority::Highest, packet2);

        // Assert
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[2]));
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[1]));
        assert!(matches!(heap.pop(), None));
    }

    #[test]
    fn push_highest_then_high_priority_packets() {
        // Arrange
        let mut heap = OutgoingPacketHeap::new();
        let packet1 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![1].into_boxed_slice());
        let packet2 = InternalPacket::new(Instant::now(), InternalReliability::Unreliable, InternalOrdering::None, None, None, vec![2].into_boxed_slice());

        // Act
        heap.push(Priority::Highest, packet1);
        heap.push(Priority::High, packet2);

        // Assert
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[1]));
        assert!(matches!(heap.pop(), Some(packet) if packet.payload() == &[2]));
        assert!(matches!(heap.pop(), None));
    }

}