//! # The Heartwood - AethelOS Kernel Library
//!
//! This library exports the core kernel functionality for use by
//! other components and for testing.

#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

/// A simple dummy allocator for now
/// In a real implementation, this would use the Mana Pool
struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Do nothing
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: DummyAllocator = DummyAllocator;

// Re-export core modules
pub mod boot;
pub mod nexus;
pub mod loom_of_fate;
pub mod mana_pool;
pub mod attunement;
pub mod vga_buffer;

// Re-export key types
pub use nexus::{Message, MessageType, MessagePriority, NexusError};
pub use loom_of_fate::{ThreadId, ThreadState, ThreadPriority, LoomError};
pub use mana_pool::{ObjectHandle, AllocationPurpose, ManaError};

/// Panic handler for lib builds
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
