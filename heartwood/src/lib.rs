//! # The Heartwood - AethelOS Kernel Library
//!
//! This library exports the core kernel functionality for use by
//! other components and for testing.

#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

// Import the production buddy allocator
use mana_pool::allocator::BuddyAllocator;

/// Global allocator for the kernel
/// Uses a production-grade buddy allocator with:
/// - Proper allocation and deallocation
/// - Interrupt-safe locking
/// - Block coalescing to minimize fragmentation
///
/// Heap region: 4MB - 8MB (4MB total)
#[global_allocator]
static GLOBAL_ALLOCATOR: BuddyAllocator = BuddyAllocator::new();

/// Initialize the global allocator
///
/// MUST be called BEFORE any heap allocations occur (including Box, Vec, etc.)
///
/// After higher-half kernel migration, the heap is located in higher-half virtual address space.
/// Physical memory at 4MB-12MB maps to virtual 0xFFFF_8000_0040_0000+.
pub fn init_global_allocator() {
    unsafe {
        // Heap in higher-half address space (top 2GB for -mcmodel=kernel)
        // Memory layout:
        //   - 1MB-3MB: Kernel code/data/bss
        //   - 4MB (0x400000): Stack top (grows DOWN towards kernel)
        //   - 12MB (0xC00000): Heap start (grows UP)
        // This gives stack 9MB of space (0x400000 to 0xC00000)
        const KERNEL_BASE: usize = 0xFFFFFFFF80000000;
        const HEAP_START: usize = KERNEL_BASE + 0xC00000;  // 12MB in top 2GB
        const HEAP_SIZE: usize = 0x800000;   // 8MB (ends at 20MB / 0x1400000)

        GLOBAL_ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }
}

/// DIAGNOSTIC: Check if allocator lock is stuck
pub fn allocator_is_locked() -> bool {
    GLOBAL_ALLOCATOR.is_locked()
}

/// DIAGNOSTIC: Force unlock the allocator
pub unsafe fn allocator_force_unlock() {
    GLOBAL_ALLOCATOR.force_unlock();
}

// Re-export core modules
pub mod boot;
pub mod nexus;
pub mod loom_of_fate;
pub mod mana_pool;
pub mod attunement;
pub mod vga_buffer;
pub mod rtl;  // Runtime library with memcpy, etc.
pub mod eldarin;  // The Eldarin Shell
pub mod wards_command;  // Security wards command
pub mod sigils_command;  // Weaver's Sigils command
pub mod permanence_command;  // Rune of Permanence command
pub mod fate_command;  // Concordance of Fates management (RBAC)
pub mod stack_protection;  // Stack canary runtime (LLVM support)
pub mod irq_safe_mutex;  // Interrupt-safe mutex primitive
pub mod vfs;  // Virtual File System layer
pub mod drivers;  // Hardware device drivers
pub mod test_programs;  // Embedded test programs

// Re-export key types
pub use nexus::{Message, MessageType, MessagePriority, NexusError};
pub use loom_of_fate::{ThreadId, ThreadState, ThreadPriority, LoomError};
pub use mana_pool::{ObjectHandle, AllocationPurpose, ManaError};
pub use vfs::{FileSystem, Path, FsError, DirEntry, FileStat};

/// Panic handler for lib builds
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
