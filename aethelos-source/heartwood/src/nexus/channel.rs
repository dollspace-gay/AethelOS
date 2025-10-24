//! Channels - The conduits through which messages flow

use super::message::Message;
use alloc::collections::VecDeque;

/// Maximum messages in a channel before backpressure
const CHANNEL_CAPACITY: usize = 256;

/// A unique identifier for a channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChannelId(pub u64);

/// A capability granting access to a channel
/// This is what processes actually hold - not raw channel IDs
#[derive(Debug, Clone, Copy)]
pub struct ChannelCapability {
    pub channel_id: ChannelId,
    pub can_send: bool,
    pub can_receive: bool,
}

impl ChannelCapability {
    pub fn new_sender(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            can_send: true,
            can_receive: false,
        }
    }

    pub fn new_receiver(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            can_send: false,
            can_receive: true,
        }
    }

    pub fn new_bidirectional(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            can_send: true,
            can_receive: true,
        }
    }
}

/// A channel is a queue of messages with priority-based ordering
pub struct Channel {
    id: ChannelId,
    messages: VecDeque<Message>,
    closed: bool,
}

impl Channel {
    pub fn new(id: ChannelId) -> Self {
        Self {
            id,
            messages: VecDeque::with_capacity(CHANNEL_CAPACITY),
            closed: false,
        }
    }

    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Send a message to this channel
    pub fn send(&mut self, message: Message) -> Result<(), ChannelError> {
        if self.closed {
            return Err(ChannelError::Closed);
        }

        if self.messages.len() >= CHANNEL_CAPACITY {
            return Err(ChannelError::Full);
        }

        // Insert message in priority order
        let priority = message.priority;
        let insert_pos = self
            .messages
            .iter()
            .position(|m| m.priority > priority)
            .unwrap_or(self.messages.len());

        self.messages.insert(insert_pos, message);

        Ok(())
    }

    /// Try to receive a message (non-blocking)
    pub fn try_receive(&mut self) -> Result<Option<Message>, ChannelError> {
        if self.closed && self.messages.is_empty() {
            return Err(ChannelError::Closed);
        }

        Ok(self.messages.pop_front())
    }

    /// Check if the channel has any messages waiting
    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Get the number of messages in the channel
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Close the channel
    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelError {
    Full,
    Closed,
    NotFound,
}
