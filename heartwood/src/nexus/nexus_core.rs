//! The Nexus Core - The beating heart of IPC

use super::channel::{Channel, ChannelId, ChannelCapability};
use super::message::Message;
use super::NexusError;
use alloc::collections::BTreeMap;

/// The maximum number of channels the Nexus can manage
const MAX_CHANNELS: usize = 4096;

/// The central Nexus manages all channels in the system
pub struct NexusCore {
    channels: BTreeMap<ChannelId, Channel>,
    next_channel_id: u64,
}

impl Default for NexusCore {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusCore {
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            next_channel_id: 1, // 0 is reserved as invalid
        }
    }

    /// Create a new bidirectional channel pair
    /// Returns two capabilities: (sender_side, receiver_side)
    pub fn create_channel(&mut self) -> Result<(ChannelCapability, ChannelCapability), NexusError> {
        if self.channels.len() >= MAX_CHANNELS {
            return Err(NexusError::OutOfChannels);
        }

        let channel_id = ChannelId(self.next_channel_id);
        self.next_channel_id += 1;

        let channel = Channel::new(channel_id);
        self.channels.insert(channel_id, channel);

        // Create bidirectional capabilities
        let cap_a = ChannelCapability::new_bidirectional(channel_id);
        let cap_b = ChannelCapability::new_bidirectional(channel_id);

        Ok((cap_a, cap_b))
    }

    /// Send a message through a channel
    pub fn send(&mut self, channel_id: ChannelId, message: Message) -> Result<(), NexusError> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or(NexusError::ChannelNotFound)?;

        channel
            .send(message)
            .map_err(|e| match e {
                super::channel::ChannelError::Full => NexusError::ChannelFull,
                super::channel::ChannelError::Closed => NexusError::ChannelClosed,
                super::channel::ChannelError::NotFound => NexusError::ChannelNotFound,
            })
    }

    /// Try to receive a message from a channel (non-blocking)
    pub fn try_receive(&mut self, channel_id: ChannelId) -> Result<Option<Message>, NexusError> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or(NexusError::ChannelNotFound)?;

        channel
            .try_receive()
            .map_err(|e| match e {
                super::channel::ChannelError::Closed => NexusError::ChannelClosed,
                super::channel::ChannelError::NotFound => NexusError::ChannelNotFound,
                super::channel::ChannelError::Full => NexusError::ChannelFull,
            })
    }

    /// Close a channel
    pub fn close_channel(&mut self, channel_id: ChannelId) -> Result<(), NexusError> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or(NexusError::ChannelNotFound)?;

        channel.close();
        Ok(())
    }

    /// Get statistics about the Nexus
    pub fn stats(&self) -> NexusStats {
        NexusStats {
            total_channels: self.channels.len(),
            active_channels: self
                .channels
                .values()
                .filter(|c| !c.is_closed())
                .count(),
            total_queued_messages: self
                .channels
                .values()
                .map(|c| c.message_count())
                .sum(),
        }
    }
}

/// Statistics about the Nexus
#[derive(Debug, Clone, Copy)]
pub struct NexusStats {
    pub total_channels: usize,
    pub active_channels: usize,
    pub total_queued_messages: usize,
}
