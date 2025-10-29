//! Global allocator integration for Rust's allocation primitives
//!
//! Now uses a production-grade buddy allocator with proper deallocation
//! and interrupt-safe locking.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use super::buddy::LockedBuddyAllocator;

/// Production buddy allocator for kernel heap
///
/// This allocator:
/// - Supports both allocation AND deallocation (unlike the old bump allocator)
/// - Is thread-safe and interrupt-safe
/// - Uses the buddy system for efficient memory management
/// - Minimizes fragmentation through block coalescing
pub struct BuddyAllocator {
    inner: LockedBuddyAllocator,
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        Self {
            inner: LockedBuddyAllocator::new(),
        }
    }

    /// Initialize the allocator with a memory region
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `heap_start` points to a valid, writable memory region
    /// - `heap_size` does not exceed the actual available memory at `heap_start`
    /// - The memory region from `heap_start` to `heap_start + heap_size` is not used by anything else
    /// - This function is called exactly once during kernel initialization
    /// - No allocations are attempted before this function is called
    /// - The memory region remains valid for the entire lifetime of the allocator
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        self.inner.init(heap_start, heap_size);
    }

    /// Get statistics about the allocator
    pub fn stats(&self) -> super::buddy::BuddyStats {
        self.inner.stats()
    }

    /// DIAGNOSTIC: Check if the allocator lock is stuck
    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }

    /// DIAGNOSTIC: Force unlock the allocator
    ///
    /// # Safety
    /// This forcibly releases the lock. Only for debugging stuck locks.
    pub unsafe fn force_unlock(&self) {
        self.inner.force_unlock();
    }
}

unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Account for alignment by allocating extra space if needed
        let size = layout.size().max(layout.align());

        match self.inner.allocate(size) {
            Some(addr) => {
                // Align the address to the required alignment
                let aligned_addr = align_up(addr, layout.align());
                aligned_addr as *mut u8
            }
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Production allocator: properly deallocates memory!
        // Memory is returned to the pool and can be reused.
        let addr = ptr as usize;
        let size = layout.size().max(layout.align());
        self.inner.deallocate(addr, size);
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
