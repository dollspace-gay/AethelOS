//! # Buddy Allocator - The Harmonious Division
//!
//! A production-grade memory allocator based on the buddy system.
//! Memory is divided into power-of-2 sized blocks that can be split
//! and coalesced efficiently.
//!
//! ## Philosophy
//! Like crystals that naturally split along perfect planes,
//! memory blocks divide and reunite in harmonious patterns.
//! Each block has a "buddy" - its perfect complement in the dance of allocation.

use core::ptr::NonNull;
use super::interrupt_lock::InterruptSafeLock;

/// Minimum block size: 64 bytes
const MIN_BLOCK_SIZE: usize = 64;

/// Maximum order: 2^MAX_ORDER * MIN_BLOCK_SIZE
/// With MAX_ORDER = 10, this gives us blocks up to 64KB
const MAX_ORDER: usize = 10;

/// Number of free lists (one for each order)
const NUM_ORDERS: usize = MAX_ORDER + 1;

/// A block in the buddy allocator
#[repr(C)]
struct Block {
    /// Next block in the free list
    next: Option<NonNull<Block>>,
}

impl Block {
    /// Create a new block at the given address
    unsafe fn new(addr: usize) -> NonNull<Block> {
        let ptr = addr as *mut Block;
        (*ptr).next = None;
        NonNull::new_unchecked(ptr)
    }

    /// Get the next block in the free list
    fn next(&self) -> Option<NonNull<Block>> {
        self.next
    }

    /// Set the next block in the free list
    fn set_next(&mut self, next: Option<NonNull<Block>>) {
        self.next = next;
    }
}

/// The Buddy Allocator
///
/// Manages memory using the buddy system algorithm.
/// Thread-safe and interrupt-safe through InterruptSafeLock.
pub struct BuddyAllocator {
    /// Free lists for each order (0..=MAX_ORDER)
    /// free_lists[i] contains blocks of size MIN_BLOCK_SIZE * 2^i
    free_lists: [Option<NonNull<Block>>; NUM_ORDERS],

    /// Start of the heap
    heap_start: usize,

    /// Size of the heap
    heap_size: usize,
}

impl BuddyAllocator {
    /// Create a new uninitialized buddy allocator
    pub const fn new() -> Self {
        Self {
            free_lists: [None; NUM_ORDERS],
            heap_start: 0,
            heap_size: 0,
        }
    }

    /// Initialize the allocator with a memory region
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `heap_start` points to a valid, writable memory region
    /// - `heap_size` does not exceed the actual available memory
    /// - The memory region is not used by anything else
    /// - This function is called exactly once
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_size = heap_size;

        // Initialize all free lists to empty
        for i in 0..NUM_ORDERS {
            self.free_lists[i] = None;
        }

        // Add the entire heap as one or more large blocks
        self.add_initial_blocks(heap_start, heap_size);
    }

    /// Add initial blocks to the free lists
    unsafe fn add_initial_blocks(&mut self, mut addr: usize, mut size: usize) {
        // Add blocks from largest to smallest
        while size >= MIN_BLOCK_SIZE {
            // Find the largest block that fits
            let order = self.size_to_order(size).min(MAX_ORDER);
            let block_size = self.order_to_size(order);

            if block_size <= size {
                // Create a block and add it to the free list
                let block = Block::new(addr);
                self.add_to_free_list(block, order);

                addr += block_size;
                size -= block_size;
            } else {
                // Try smaller order
                break;
            }
        }
    }

    /// Allocate a block of at least `size` bytes
    ///
    /// Returns the address of the allocated block (user data, after pre-canary),
    /// or None if out of memory.
    ///
    /// Memory layout:
    /// ```
    /// [PRE_CANARY (8B)] [USER DATA (size)] [POST_CANARY (8B)]
    /// ^                  ^
    /// block address      returned address
    /// ```
    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        if size == 0 {
            return None;
        }

        // Check if heap canaries are enabled
        let canaries_enabled = super::heap_canaries::are_enabled();

        // Add space for heap canaries only if enabled
        let total_size = if canaries_enabled {
            size + super::heap_canaries::TOTAL_CANARY_OVERHEAD
        } else {
            size
        };

        // Find the order needed for this size
        let needed_order = self.size_to_order(total_size);
        if needed_order > MAX_ORDER {
            return None; // Allocation too large
        }

        // Find the smallest available block
        let alloc_order = self.find_free_block(needed_order)?;

        // Remove block from free list
        let block = self.remove_from_free_list(alloc_order)?;

        // Split block if it's larger than needed
        self.split_block(block, alloc_order, needed_order);

        let block_addr = block.as_ptr() as usize;

        // Write heap canaries around the allocation (only if enabled)
        if canaries_enabled {
            unsafe {
                super::heap_canaries::write_canaries(block_addr, size);
            }
            // Return pointer to user data (after pre-canary)
            Some(block_addr + super::heap_canaries::CANARY_SIZE)
        } else {
            // Return block address directly (no canary offset)
            Some(block_addr)
        }
    }

    /// Deallocate a block
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `addr` was returned by a previous call to `allocate` (points to user data)
    /// - `size` matches the size passed to `allocate` (user data size, not including canaries)
    /// - The block has not already been deallocated
    pub unsafe fn deallocate(&mut self, addr: usize, size: usize) {
        if size == 0 {
            return;
        }

        // Check if heap canaries are enabled
        let canaries_enabled = super::heap_canaries::are_enabled();

        let (block_addr, total_size) = if canaries_enabled {
            // Calculate the actual block address (before pre-canary)
            let blk_addr = addr - super::heap_canaries::CANARY_SIZE;

            // Verify heap canaries before deallocation
            if !super::heap_canaries::verify_canaries(blk_addr, size) {
                panic!("◈ HEAP CORRUPTION: Canary violation detected during deallocation!\n\
                       \n\
                       Address: 0x{:016x}\n\
                       Size: {} bytes\n\
                       \n\
                       The Weaver's Sigil has detected heap buffer overflow.\n\
                       This allocation's protective canaries were corrupted.", addr, size);
            }

            // Calculate total size including canaries
            (blk_addr, size + super::heap_canaries::TOTAL_CANARY_OVERHEAD)
        } else {
            // No canaries - addr is the block address, size is actual size
            (addr, size)
        };

        let order = self.size_to_order(total_size);
        if order > MAX_ORDER {
            return;
        }

        // Create a block at the actual block address
        let block = Block::new(block_addr);

        // Try to coalesce with buddy
        self.coalesce_and_free(block, order);
    }

    /// Find a free block of at least the given order
    ///
    /// Returns the order of the block found (may be larger than requested)
    fn find_free_block(&self, min_order: usize) -> Option<usize> {
        for order in min_order..=MAX_ORDER {
            if self.free_lists[order].is_some() {
                return Some(order);
            }
        }
        None
    }

    /// Remove a block from the free list of the given order
    fn remove_from_free_list(&mut self, order: usize) -> Option<NonNull<Block>> {
        let block = self.free_lists[order]?;

        unsafe {
            // Update free list to point to next block
            self.free_lists[order] = block.as_ref().next();
        }

        Some(block)
    }

    /// Add a block to the free list of the given order
    fn add_to_free_list(&mut self, mut block: NonNull<Block>, order: usize) {
        unsafe {
            // Set this block's next to current head
            block.as_mut().set_next(self.free_lists[order]);

            // Make this block the new head
            self.free_lists[order] = Some(block);
        }
    }

    /// Split a block down to the target order
    ///
    /// If the block is larger than needed, split it and add the
    /// unused parts to the appropriate free lists
    fn split_block(&mut self, block: NonNull<Block>, current_order: usize, target_order: usize) {
        let mut order = current_order;
        let addr = block.as_ptr() as usize;

        while order > target_order {
            order -= 1;
            let block_size = self.order_to_size(order);

            // The "buddy" is the second half of the split
            let buddy_addr = addr + block_size;

            unsafe {
                let buddy = Block::new(buddy_addr);
                self.add_to_free_list(buddy, order);
            }
        }
    }

    /// Coalesce a block with its buddy and add to free list
    ///
    /// Recursively coalesces with buddies until no more coalescing is possible
    fn coalesce_and_free(&mut self, block: NonNull<Block>, mut order: usize) {
        let mut current_addr = block.as_ptr() as usize;

        // Try to coalesce with buddies
        while order < MAX_ORDER {
            let buddy_addr = self.buddy_address(current_addr, order);

            // Check if buddy is in our heap
            if !self.is_valid_address(buddy_addr, order) {
                break;
            }

            // Try to find and remove buddy from free list
            if let Some(buddy_block) = self.find_and_remove_buddy(buddy_addr, order) {
                // Coalesce: move to lower address and increase order
                current_addr = current_addr.min(buddy_block.as_ptr() as usize);
                order += 1;
            } else {
                // Buddy is not free, stop coalescing
                break;
            }
        }

        // Add the (possibly coalesced) block to free list
        unsafe {
            let final_block = Block::new(current_addr);
            self.add_to_free_list(final_block, order);
        }
    }

    /// Find and remove a specific block from a free list
    fn find_and_remove_buddy(&mut self, addr: usize, order: usize) -> Option<NonNull<Block>> {
        let mut current = self.free_lists[order]?;
        let mut prev: Option<NonNull<Block>> = None;

        unsafe {
            loop {
                if current.as_ptr() as usize == addr {
                    // Found it! Remove from list
                    if let Some(mut prev_block) = prev {
                        prev_block.as_mut().set_next(current.as_ref().next());
                    } else {
                        self.free_lists[order] = current.as_ref().next();
                    }
                    return Some(current);
                }

                // Move to next block
                prev = Some(current);
                current = current.as_ref().next()?;
            }
        }
    }

    /// Calculate the buddy address for a block
    ///
    /// The buddy of a block at address A with order O is at:
    /// A XOR (size of block)
    fn buddy_address(&self, addr: usize, order: usize) -> usize {
        let block_size = self.order_to_size(order);
        addr ^ block_size
    }

    /// Check if an address is valid for a block of the given order
    fn is_valid_address(&self, addr: usize, order: usize) -> bool {
        let block_size = self.order_to_size(order);
        addr >= self.heap_start && addr + block_size <= self.heap_start + self.heap_size
    }

    /// Convert a size to the minimum order needed
    fn size_to_order(&self, size: usize) -> usize {
        let size = size.max(MIN_BLOCK_SIZE);
        let blocks = (size + MIN_BLOCK_SIZE - 1) / MIN_BLOCK_SIZE;

        // Find the order: 2^order >= blocks
        let mut order = 0;
        let mut power = 1;
        while power < blocks {
            order += 1;
            power *= 2;
        }
        order
    }

    /// Convert an order to a block size
    fn order_to_size(&self, order: usize) -> usize {
        MIN_BLOCK_SIZE << order // MIN_BLOCK_SIZE * 2^order
    }

    /// Get statistics about the allocator
    pub fn stats(&self) -> BuddyStats {
        let mut free_blocks = 0;
        let mut free_bytes = 0;

        for order in 0..=MAX_ORDER {
            let mut count = 0;
            let mut current = self.free_lists[order];

            unsafe {
                while let Some(block) = current {
                    count += 1;
                    current = block.as_ref().next();
                }
            }

            free_blocks += count;
            free_bytes += count * self.order_to_size(order);
        }

        BuddyStats {
            total_bytes: self.heap_size,
            free_bytes,
            used_bytes: self.heap_size - free_bytes,
            free_blocks,
        }
    }
}

/// Statistics about the buddy allocator
#[derive(Debug, Clone, Copy)]
pub struct BuddyStats {
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub used_bytes: usize,
    pub free_blocks: usize,
}

/// Thread-safe and interrupt-safe wrapper for BuddyAllocator
pub struct LockedBuddyAllocator {
    inner: InterruptSafeLock<BuddyAllocator>,
}

impl LockedBuddyAllocator {
    pub const fn new() -> Self {
        Self {
            inner: InterruptSafeLock::new(BuddyAllocator::new()),
        }
    }

    /// Initialize the allocator
    ///
    /// # Safety
    /// Same requirements as BuddyAllocator::init
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        self.inner.lock().init(heap_start, heap_size);
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize) -> Option<usize> {
        self.inner.lock().allocate(size)
    }

    /// Deallocate memory
    ///
    /// # Safety
    /// Same requirements as BuddyAllocator::deallocate
    pub unsafe fn deallocate(&self, addr: usize, size: usize) {
        self.inner.lock().deallocate(addr, size);
    }

    /// Get statistics
    pub fn stats(&self) -> BuddyStats {
        self.inner.lock().stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_to_order() {
        let allocator = BuddyAllocator::new();

        // MIN_BLOCK_SIZE = 64
        assert_eq!(allocator.size_to_order(1), 0);    // 1 byte -> order 0 (64 bytes)
        assert_eq!(allocator.size_to_order(64), 0);   // 64 bytes -> order 0
        assert_eq!(allocator.size_to_order(65), 1);   // 65 bytes -> order 1 (128 bytes)
        assert_eq!(allocator.size_to_order(128), 1);  // 128 bytes -> order 1
        assert_eq!(allocator.size_to_order(129), 2);  // 129 bytes -> order 2 (256 bytes)
    }

    #[test]
    fn test_order_to_size() {
        let allocator = BuddyAllocator::new();

        assert_eq!(allocator.order_to_size(0), 64);     // 64 * 2^0
        assert_eq!(allocator.order_to_size(1), 128);    // 64 * 2^1
        assert_eq!(allocator.order_to_size(2), 256);    // 64 * 2^2
        assert_eq!(allocator.order_to_size(10), 65536); // 64 * 2^10 = 64KB
    }

    #[test]
    fn test_buddy_address() {
        let allocator = BuddyAllocator::new();

        // For order 0 (64 bytes), buddies differ by 64
        assert_eq!(allocator.buddy_address(0x1000, 0), 0x1000 ^ 64);

        // For order 1 (128 bytes), buddies differ by 128
        assert_eq!(allocator.buddy_address(0x1000, 1), 0x1000 ^ 128);
    }
}
