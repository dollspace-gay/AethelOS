# AethelOS Production Readiness Plan

This document outlines the implementation plan for addressing TODOs and "In a real OS" comments found in the codebase. These improvements are necessary to transition from a demonstration OS to a production-ready system.

## Overview

Found 4 critical areas requiring implementation:
1. Preemptive Multitasking
2. Interrupt-safe Statistics
3. Production Memory Allocator
4. Memory Deallocation

---

## 1. Preemptive Multitasking

**Location**: [heartwood/src/attunement/idt.rs:47](../heartwood/src/attunement/idt.rs#L47)

**Current State**:
- System uses cooperative multitasking (threads explicitly call `yield_now()`)
- Timer interrupt handler does NOT trigger context switches
- Preemptive context switching was disabled due to issues with interrupting critical sections (e.g., Drop implementations)

**Required Changes**:

### Phase 1: Critical Section Protection
- Implement interrupt-safe spinlocks with interrupt disable/enable
- Add `cli/sti` wrapper types that disable/enable interrupts in critical sections
- Audit all critical sections (especially in scheduler, allocator, and I/O drivers)
- Replace existing `spin::Mutex` with interrupt-safe mutexes where needed

### Phase 2: Context Switch Safety
- Design interrupt-safe context switching mechanism
- Ensure stack switching is atomic and safe during interrupts
- Implement proper save/restore of all CPU registers during preemptive switches
- Add interrupt nesting counter to prevent context switches during nested interrupts

### Phase 3: Scheduler Integration
- Modify timer interrupt handler to call scheduler's context switch
- Implement time quantum/slice management (e.g., 10ms per thread)
- Add preemption flags to thread control blocks
- Implement priority-based preemption (higher priority threads can preempt lower ones)

### Phase 4: Testing & Validation
- Test with compute-intensive threads that don't yield
- Verify critical sections are not interrupted mid-operation
- Stress test with many threads competing for CPU
- Validate that Drop implementations complete without interruption

**Dependencies**:
- Must complete interrupt-safe allocator first (Item #3)
- Requires interrupt-safe locks throughout the codebase

**Estimated Complexity**: High
**Priority**: Medium (nice to have, but cooperative multitasking works for now)

---

## 2. Interrupt-Safe Statistics

**Location**: [heartwood/src/loom_of_fate/system_threads.rs:263](../heartwood/src/loom_of_fate/system_threads.rs#L263)

**Current State**:
- Stats display is commented out in welcome message
- Calling `stats()` locks the scheduler
- If a timer interrupt fires while the lock is held, deadlock occurs

**Required Changes**:

### Approach A: Interrupt-Safe Stats Function (Recommended)
1. Create a lock-free or interrupt-safe stats snapshot mechanism
2. Use atomic operations to read thread counts and states
3. Disable interrupts briefly while copying stats to a local buffer
4. Format and display stats from the local buffer (no locks held)

**Implementation**:
```rust
pub struct StatsSnapshot {
    thread_count: usize,
    running_threads: usize,
    sleeping_threads: usize,
    // ... other stats
}

pub fn get_stats_snapshot() -> StatsSnapshot {
    // Disable interrupts temporarily
    let _guard = InterruptDisableGuard::new();

    // Quickly copy stats without complex locking
    let loom = unsafe { &*LOOM.as_ptr() };
    StatsSnapshot {
        thread_count: loom.threads.len(),
        // ... copy other stats
    }
    // Interrupts re-enabled when guard drops
}
```

### Approach B: Cache Stats Before Threads Start
1. Calculate and store stats before enabling interrupts
2. Display cached stats in welcome message
3. Simpler but less dynamic (stats won't update)

**Recommended**: Approach A for more dynamic and useful stats

**Dependencies**: None (can be implemented independently)
**Estimated Complexity**: Low
**Priority**: High (improves user experience and system visibility)

---

## 3. Production Memory Allocator

**Location**: [heartwood/src/mana_pool/allocator.rs:43](../heartwood/src/mana_pool/allocator.rs#L43)

**Current State**:
- Using simple bump allocator
- Never reclaims memory
- Not thread-safe
- Will exhaust heap quickly under real workloads

**Required Changes**:

### Phase 1: Choose Allocator Strategy

**Option A: Buddy Allocator**
- Splits memory into power-of-2 sized blocks
- Fast allocation and deallocation
- Some internal fragmentation
- Good for kernel use

**Option B: Slab Allocator**
- Pre-allocates objects of common sizes
- Extremely fast for fixed-size allocations
- Reduces fragmentation
- Ideal for kernel objects (thread blocks, file handles, etc.)

**Option C: Hybrid (Recommended)**
- Use slab allocator for common kernel structures
- Use buddy allocator for general-purpose allocations
- Best of both worlds

### Phase 2: Implement Buddy Allocator

1. **Data Structures**:
   - Free list for each order (e.g., 4KB, 8KB, 16KB, ... 1MB)
   - Bitmap or linked list to track free/allocated blocks
   - Metadata about each block's order

2. **Core Operations**:
   - `alloc()`: Find smallest suitable block, split if needed
   - `dealloc()`: Coalesce with buddy blocks when freed
   - `split_block()`: Split larger blocks into smaller ones
   - `coalesce()`: Merge adjacent free blocks

3. **Thread Safety**:
   - Wrap allocator in interrupt-safe spinlock
   - Disable interrupts during allocation/deallocation
   - Keep critical sections as short as possible

### Phase 3: Implement Slab Allocator (Optional)

1. Create object caches for common sizes:
   - Thread control blocks
   - File descriptors
   - Network buffers
   - Page tables

2. Each slab contains:
   - Array of objects
   - Free list of available objects
   - Reference to next slab

3. Fast path: Pop from free list (O(1))
4. Slow path: Allocate new slab from buddy allocator

### Phase 4: Integration

1. Replace `BumpAllocator` with new allocator
2. Update `GlobalAlloc` implementation
3. Add allocation statistics and debugging
4. Test with existing code (should be drop-in replacement)

**Dependencies**: None (can be implemented independently)
**Estimated Complexity**: Medium-High
**Priority**: High (critical for production use)

---

## 4. Memory Deallocation

**Location**: [heartwood/src/mana_pool/allocator.rs:61](../heartwood/src/mana_pool/allocator.rs#L61)

**Current State**:
- `dealloc()` is a no-op
- Memory is never reclaimed
- Will leak memory for any temporary allocations

**Required Changes**:

This is directly addressed by implementing the production allocator (#3 above). The buddy/slab allocators both support proper deallocation.

**Implementation Notes**:
- Buddy allocator: Mark block as free, attempt to coalesce with buddy
- Slab allocator: Return object to slab's free list
- Must handle double-free detection (debug builds)
- Consider memory poisoning in debug mode to catch use-after-free

**Dependencies**: Requires Item #3 (Production Allocator)
**Estimated Complexity**: Medium (part of allocator implementation)
**Priority**: High (same as #3)

---

## Implementation Roadmap

### Phase 1: Foundation (High Priority)
1. **Interrupt-Safe Stats** (Item #2)
   - Low complexity, immediate user benefit
   - No dependencies
   - Estimated time: 2-4 hours

2. **Production Memory Allocator** (Items #3 & #4)
   - Start with buddy allocator
   - Add thread safety with interrupt-safe locks
   - Estimated time: 2-3 days

### Phase 2: Advanced Features (Medium Priority)
3. **Slab Allocator** (Optional enhancement to #3)
   - Build on top of buddy allocator
   - Optimize for common kernel objects
   - Estimated time: 1-2 days

4. **Preemptive Multitasking** (Item #1)
   - Requires allocator and locks to be interrupt-safe first
   - Extensive testing needed
   - Estimated time: 3-5 days

### Phase 3: Optimization & Polish
- Add memory allocation statistics
- Implement memory pressure handling
- Add OOM (out-of-memory) handler
- Performance tuning and profiling

---

## Testing Strategy

### For Each Item:

1. **Unit Tests**:
   - Test allocator operations (alloc, free, coalesce)
   - Test stats snapshot under various conditions
   - Test context switching edge cases

2. **Integration Tests**:
   - Run existing demos with new allocator
   - Verify threads still work correctly
   - Test under memory pressure

3. **Stress Tests**:
   - Allocate/free memory rapidly
   - Many threads competing for resources
   - Long-running system stability tests

4. **Regression Tests**:
   - Ensure existing functionality still works
   - No new deadlocks or race conditions introduced

---

## Success Criteria

### Item #1: Preemptive Multitasking
- [ ] Compute-intensive thread can be preempted by timer
- [ ] No deadlocks in critical sections
- [ ] Drop implementations complete without interruption
- [ ] System remains stable under heavy load

### Item #2: Interrupt-Safe Stats
- [ ] Stats display in welcome message without deadlock
- [ ] Stats update correctly even with interrupts enabled
- [ ] No performance degradation

### Item #3 & #4: Production Allocator
- [ ] Can allocate and free memory correctly
- [ ] Memory is reclaimed and reused
- [ ] Thread-safe and interrupt-safe
- [ ] No memory corruption or leaks
- [ ] Performance acceptable (< 1us for typical allocations)
- [ ] Works as drop-in replacement for bump allocator

---

## References

### Allocator Resources
- "The Buddy System" - Knuth, TAOCP Vol 1
- Linux kernel slab allocator (SLUB/SLOB)
- [OSDev Wiki: Page Frame Allocation](https://wiki.osdev.org/Page_Frame_Allocation)

### Preemption Resources
- "Operating Systems: Three Easy Pieces" - Chapter on Concurrency
- [OSDev Wiki: Interrupt Service Routines](https://wiki.osdev.org/Interrupt_Service_Routines)
- x86_64 interrupt handling best practices

### Synchronization Resources
- "The Art of Multiprocessor Programming" - Herlihy & Shavit
- Interrupt-safe locking patterns
- Spinlock implementation best practices

---

## Notes

- All implementations should follow AethelOS naming conventions (e.g., "Mana Pool" for memory)
- Maintain the poetic/philosophical tone in documentation
- Consider creating new modules:
  - `mana_pool::buddy` for buddy allocator
  - `mana_pool::slab` for slab allocator
  - `attunement::preemption` for preemptive scheduling
- Add extensive comments explaining design decisions
