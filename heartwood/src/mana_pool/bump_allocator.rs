//! Simple Bump Allocator for Testing
//!
//! This is a minimal allocator that just bumps a pointer forward.
//! Used to test if the buddy allocator has issues under Limine.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr::null_mut;

pub struct BumpAllocator {
    heap_start: AtomicUsize,
    heap_end: AtomicUsize,
    next: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: AtomicUsize::new(0),
            heap_end: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, heap_start: usize, heap_size: usize) {
        self.heap_start.store(heap_start, Ordering::Relaxed);
        self.heap_end.store(heap_start + heap_size, Ordering::Relaxed);
        self.next.store(heap_start, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Serial marker: allocation start
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'>', options(nomem, nostack, preserves_flags));

        let size = layout.size();
        let align = layout.align();

        // Get current position
        let mut current = self.next.load(Ordering::Relaxed);

        // Align up
        let aligned = (current + align - 1) & !(align - 1);
        let new_next = aligned + size;

        // Check if we have space
        if new_next > self.heap_end.load(Ordering::Relaxed) {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'X', options(nomem, nostack, preserves_flags));
            return null_mut();
        }

        // Update next pointer
        self.next.store(new_next, Ordering::Relaxed);

        // Serial marker: allocation success
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'<', options(nomem, nostack, preserves_flags));

        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate individual allocations
        // Serial marker: dealloc (ignored)
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'~', options(nomem, nostack, preserves_flags));
    }
}
