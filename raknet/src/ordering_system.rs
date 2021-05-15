use std::collections::HashMap;

use crate::{constants::NUMBER_OF_ORDERING_CHANNELS, ordering_channel::OrderingChannel};

pub struct OrderingSystem {
    channels: HashMap<u8, OrderingChannel>,
}

impl OrderingSystem {
    pub fn new() -> Self {
        OrderingSystem {
            channels: HashMap::new(),      
        }
    }

    pub fn get_channel(&mut self, channel_index: u8) -> Option<&mut OrderingChannel> {
        if channel_index < NUMBER_OF_ORDERING_CHANNELS {
            Some(self.channels.entry(channel_index).or_insert_with(|| OrderingChannel::new()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::NUMBER_OF_ORDERING_CHANNELS;
    use super::OrderingSystem;

    #[test]
    fn get_channel_valid_channel_index() {
        // Arrange
        let mut ordering_system = OrderingSystem::new();

        // Act
        let channel = ordering_system.get_channel(NUMBER_OF_ORDERING_CHANNELS - 1);

        // Assert
        assert!(matches!(channel, Some(_channel)));
    }

    #[test]
    fn get_channel_invalid_channel_index() {
        // Arrange
        let mut ordering_system = OrderingSystem::new();

        // Act
        let channel = ordering_system.get_channel(NUMBER_OF_ORDERING_CHANNELS);

        // Assert
        assert!(matches!(channel, None));
    }    
}