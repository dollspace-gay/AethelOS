//! # The Nexus
//!
//! The Inter-Process Communication core of AethelOS.
//! The Nexus is the primary means by which all components of the system
//! communicate - from kernel services to user-space Groves.
//!
//! ## Philosophy
//! The Nexus does not force communication; it channels it.
//! Messages flow like water through natural paths, finding their destination
//! through intent rather than rigid addressing.
//!
//! ## Architecture
//! - Asynchronous, non-blocking message passing
//! - Capability-based addressing (no raw process IDs)
//! - Priority-aware delivery (harmony-based routing)
//! - Zero-copy where possible (messages live in the Mana Pool)

pub mod message;
pub mod nexus_core;
pub mod channel;

pub use message::{Message, MessageType, MessagePriority};
pub use nexus_core::NexusCore;
pub use channel::{Channel, ChannelId, ChannelCapability};

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref NEXUS: Mutex<NexusCore> = Mutex::new(NexusCore::new());
}

/// Initialize the Nexus
pub fn init() {
    let nexus = NEXUS.lock();
    // Nexus initialization happens on construction
    drop(nexus);
}

/// Send a message through the Nexus
pub fn send(channel: ChannelId, message: Message) -> Result<(), NexusError> {
    NEXUS.lock().send(channel, message)
}

/// Receive a message from a channel (non-blocking)
pub fn try_receive(channel: ChannelId) -> Result<Option<Message>, NexusError> {
    NEXUS.lock().try_receive(channel)
}

/// Create a new bidirectional channel
pub fn create_channel() -> Result<(ChannelCapability, ChannelCapability), NexusError> {
    NEXUS.lock().create_channel()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexusError {
    ChannelNotFound,
    ChannelFull,
    ChannelClosed,
    InvalidCapability,
    OutOfChannels,
}
