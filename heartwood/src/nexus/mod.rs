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

use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;

// Using InterruptSafeLock to prevent deadlocks during preemptive multitasking
static mut NEXUS: MaybeUninit<InterruptSafeLock<NexusCore>> = MaybeUninit::uninit();
static mut NEXUS_INITIALIZED: bool = false;

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Initialize the Nexus
pub fn init() {
    unsafe {
        serial_out(b'n'); // Nexus init started
        let nexus_core = NexusCore::new();
        serial_out(b'x'); // NexusCore::new() complete

        // Create interrupt-safe lock and write to static
        serial_out(b's'); // Before InterruptSafeLock::new
        let lock = InterruptSafeLock::new(nexus_core);
        serial_out(b'u'); // After InterruptSafeLock::new

        core::ptr::write(core::ptr::addr_of_mut!(NEXUS).cast(), lock);
        serial_out(b'!'); // Written to static

        NEXUS_INITIALIZED = true;
    }
}

/// Get reference to NEXUS (assumes initialized)
unsafe fn get_nexus() -> &'static InterruptSafeLock<NexusCore> {
    &*core::ptr::addr_of!(NEXUS).cast::<InterruptSafeLock<NexusCore>>()
}

/// Send a message through the Nexus
pub fn send(channel: ChannelId, message: Message) -> Result<(), NexusError> {
    unsafe { get_nexus().lock().send(channel, message) }
}

/// Receive a message from a channel (non-blocking)
pub fn try_receive(channel: ChannelId) -> Result<Option<Message>, NexusError> {
    unsafe { get_nexus().lock().try_receive(channel) }
}

/// Create a new bidirectional channel
pub fn create_channel() -> Result<(ChannelCapability, ChannelCapability), NexusError> {
    unsafe { get_nexus().lock().create_channel() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexusError {
    ChannelNotFound,
    ChannelFull,
    ChannelClosed,
    InvalidCapability,
    OutOfChannels,
}
