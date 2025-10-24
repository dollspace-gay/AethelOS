//! # Stack Allocation
//!
//! Each thread needs its own stack - a space for its local thoughts.
//! We allocate stacks from the heap, giving each thread room to breathe.
//!
//! ## Philosophy
//! A thread's stack is its private sanctuary. We provide generous space
//! (default 64KB) so threads never feel cramped or anxious.

use alloc::alloc::{alloc, dealloc, Layout};
use core::ptr::NonNull;

/// Default stack size: 64 KB
/// This is generous enough for most threads while being conservative with memory
pub const DEFAULT_STACK_SIZE: usize = 64 * 1024;

/// Minimum stack size: 4 KB (one page)
pub const MIN_STACK_SIZE: usize = 4 * 1024;

/// Maximum stack size: 1 MB
pub const MAX_STACK_SIZE: usize = 1024 * 1024;

/// A thread's stack allocation
pub struct Stack {
    bottom: NonNull<u8>,
    size: usize,
}

// Safety: Stack pointers can be safely sent between threads
// as they represent heap-allocated memory
unsafe impl Send for Stack {}
unsafe impl Sync for Stack {}

impl Stack {
    /// Allocate a new stack with the default size
    pub fn new() -> Option<Self> {
        Self::with_size(DEFAULT_STACK_SIZE)
    }

    /// Allocate a new stack with a specific size
    ///
    /// # Arguments
    /// * `size` - Size in bytes (will be clamped to MIN/MAX and aligned to 16 bytes)
    ///
    /// # Returns
    /// Some(Stack) if allocation succeeds, None otherwise
    pub fn with_size(size: usize) -> Option<Self> {
        // Clamp size to valid range and align to 16 bytes
        let size = size.clamp(MIN_STACK_SIZE, MAX_STACK_SIZE);
        let size = (size + 15) & !15; // Align to 16 bytes

        // Create layout for allocation
        let layout = Layout::from_size_align(size, 16).ok()?;

        // Allocate the stack
        let ptr = unsafe { alloc(layout) };

        NonNull::new(ptr).map(|bottom| Stack { bottom, size })
    }

    /// Get the bottom (low address) of the stack
    pub fn bottom(&self) -> u64 {
        self.bottom.as_ptr() as u64
    }

    /// Get the top (high address) of the stack
    ///
    /// The stack grows downward, so this is bottom + size
    pub fn top(&self) -> u64 {
        self.bottom() + self.size as u64
    }

    /// Get the size of the stack in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if an address is within this stack
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.bottom() && addr < self.top()
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // Deallocate the stack when it's no longer needed
        let layout = Layout::from_size_align(self.size, 16)
            .expect("Invalid layout during stack deallocation");

        unsafe {
            dealloc(self.bottom.as_ptr(), layout);
        }
    }
}

/// Guard page support (for future implementation)
///
/// A guard page is a page marked as non-accessible that sits at the bottom
/// of the stack. If a stack overflow occurs, accessing the guard page will
/// trigger a page fault, allowing us to detect the overflow gracefully.
#[allow(dead_code)]
pub struct GuardedStack {
    stack: Stack,
    guard_page_addr: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_allocation() {
        let stack = Stack::new().expect("Failed to allocate stack");
        assert_eq!(stack.size(), DEFAULT_STACK_SIZE);
        assert!(stack.top() > stack.bottom());
        assert_eq!(stack.top() - stack.bottom(), DEFAULT_STACK_SIZE as u64);
    }

    #[test]
    fn test_custom_size() {
        let stack = Stack::with_size(8192).expect("Failed to allocate");
        assert_eq!(stack.size(), 8192);
    }

    #[test]
    fn test_size_clamping() {
        // Too small - should clamp to MIN_STACK_SIZE
        let stack = Stack::with_size(100).expect("Failed to allocate");
        assert_eq!(stack.size(), MIN_STACK_SIZE);

        // Too large - should clamp to MAX_STACK_SIZE
        let stack = Stack::with_size(10 * 1024 * 1024).expect("Failed to allocate");
        assert_eq!(stack.size(), MAX_STACK_SIZE);
    }

    #[test]
    fn test_contains() {
        let stack = Stack::new().expect("Failed to allocate");
        let bottom = stack.bottom();
        let top = stack.top();

        assert!(stack.contains(bottom));
        assert!(stack.contains(bottom + 100));
        assert!(stack.contains(top - 1));
        assert!(!stack.contains(top));
        assert!(!stack.contains(bottom - 1));
    }
}
