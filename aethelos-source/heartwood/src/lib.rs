//! # The Heartwood - AethelOS Kernel Library
//!
//! This library exports the core kernel functionality for use by
//! other components and for testing.

#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

/// Simple bump allocator for kernel heap
/// Allocates from a fixed 1MB region starting at 3MB
struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: core::sync::atomic::AtomicUsize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0x400000,  // 4MB (well above kernel and stack)
            heap_end: 0x800000,    // 8MB (4MB heap - plenty of space)
            next: core::sync::atomic::AtomicUsize::new(0x400000),
        }
    }
}

/// Helper to write to serial for debugging allocator
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        serial_out(b'['); // Entering alloc

        let size = layout.size();
        let align = layout.align();

        // Align the allocation
        let current = self.next.load(core::sync::atomic::Ordering::Relaxed);
        let aligned = (current + align - 1) & !(align - 1);
        let new_next = aligned + size;

        if new_next > self.heap_end {
            serial_out(b'!'); // Out of memory!
            return core::ptr::null_mut(); // Out of memory
        }

        self.next.store(new_next, core::sync::atomic::Ordering::Relaxed);
        serial_out(b']'); // Successful alloc
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
        serial_out(b'-'); // Dealloc called (ignored)
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator::new();

// Re-export core modules
pub mod boot;
pub mod nexus;
pub mod loom_of_fate;
pub mod mana_pool;
pub mod attunement;
pub mod vga_buffer;
pub mod rtl;  // Runtime library with memcpy, etc.
pub mod eldarin;  // The Eldarin Shell
pub mod irq_safe_mutex;  // Interrupt-safe mutex primitive

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
