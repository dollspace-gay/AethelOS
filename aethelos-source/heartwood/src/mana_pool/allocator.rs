//! Global allocator integration for Rust's allocation primitives

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

/// A simple bump allocator for kernel heap
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
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
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Simple bump allocation - not suitable for production
        // In a real OS, use a proper allocator like buddy or slab
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > self.heap_end {
            null_mut()
        } else {
            let ptr = alloc_start as *mut u8;
            // Note: This is not thread-safe. Would need atomic ops in real implementation
            ptr
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
        // In a real OS, implement proper deallocation
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
