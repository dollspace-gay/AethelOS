# Preemptive Multitasking Implementation Plan

This document provides a detailed, step-by-step plan for implementing preemptive multitasking in AethelOS. Preemptive multitasking is complex because it introduces race conditions and requires careful synchronization.

## Overview

**Current State:**
- Cooperative multitasking (threads call `yield_now()` explicitly)
- Timer interrupts do NOT trigger context switches
- Safe but threads can monopolize CPU if they don't yield

**Goal:**
- Preemptive multitasking with timer-driven context switches
- Threads are interrupted and switched automatically
- Must maintain system stability and avoid deadlocks

---

## Why Preemptive Multitasking is Hard

### Challenge 1: Critical Sections
**Problem:** If we preempt a thread while it's in a critical section (holding a lock), another thread might try to acquire the same lock → deadlock.

**Example:**
```
Thread A: Acquires lock → Timer interrupt → Context switch to Thread B
Thread B: Tries to acquire same lock → Deadlock!
```

**Solution:** Use interrupt-safe locks that disable interrupts while held (already implemented in `InterruptSafeLock`).

### Challenge 2: Context Switch Atomicity
**Problem:** Context switching involves multiple steps (save registers, switch stack, restore registers). If interrupted mid-switch, corruption occurs.

**Solution:** Disable interrupts during context switches, use atomic operations.

### Challenge 3: Drop Implementations
**Problem:** Rust Drop implementations can be called at any time. If preempted during Drop, resources may leak or corrupt.

**Example:**
```rust
impl Drop for MyStruct {
    fn drop(&mut self) {
        // If preempted here, lock may not be released!
        self.lock.unlock();
    }
}
```

**Solution:** Make Drop implementations interrupt-safe, or disable preemption during Drop.

### Challenge 4: Stack Safety
**Problem:** If we preempt while stack is in an invalid state (mid-push, mid-pop), corruption occurs.

**Solution:** Stack operations are atomic at instruction level, but we must ensure proper alignment.

---

## Implementation Strategy

We'll implement preemptive multitasking in **5 phases**, each building on the previous:

### Phase 1: Audit and Fix Critical Sections ⚠️ CRITICAL
### Phase 2: Implement Preemption Control
### Phase 3: Timer-Driven Context Switching
### Phase 4: Testing and Debugging
### Phase 5: Optimization and Tuning

---

## Phase 1: Audit and Fix Critical Sections

**Goal:** Ensure all locks are interrupt-safe

### Step 1.1: Identify All Locks in the System

Search for all mutex/spinlock usage:
- `spin::Mutex` (NOT interrupt-safe)
- `InterruptSafeLock` (IS interrupt-safe)
- Custom locks

**Files to audit:**
- `mana_pool/mod.rs` - MANA_POOL mutex
- `loom_of_fate/mod.rs` - LOOM mutex
- `nexus/mod.rs` - NEXUS mutex
- `vga_buffer.rs` - WRITER mutex
- `eldarin.rs` - BUFFER mutex
- Any other modules with static mutexes

### Step 1.2: Convert to Interrupt-Safe Locks

For each lock found, determine if it needs to be interrupt-safe:

**Needs to be interrupt-safe if:**
- Used in both regular code AND interrupt handlers
- Holds critical system state (scheduler, allocator)
- Guards shared resources

**Strategy:**
```rust
// BEFORE (unsafe with preemption)
static LOCK: Mutex<Data> = Mutex::new(Data::new());

// AFTER (safe with preemption)
static LOCK: InterruptSafeLock<Data> = InterruptSafeLock::new(Data::new());
```

**Files to modify:**
1. Replace `spin::Mutex` with `InterruptSafeLock` in:
   - `mana_pool/mod.rs` (MANA_POOL)
   - `loom_of_fate/mod.rs` (LOOM)
   - `nexus/mod.rs` (NEXUS)
   - `vga_buffer.rs` (WRITER)
   - `eldarin.rs` (BUFFER)

2. Test that system still works with new locks

**Verification:**
- [ ] All system locks use `InterruptSafeLock`
- [ ] Build succeeds
- [ ] System boots without deadlock
- [ ] Stats display works

---

## Phase 2: Implement Preemption Control

**Goal:** Add ability to enable/disable preemption

### Step 2.1: Add Preemption State to Scheduler

Add to `Scheduler` struct:
```rust
pub struct Scheduler {
    // ... existing fields ...

    /// Is preemptive scheduling enabled?
    preemption_enabled: bool,

    /// Time quantum in timer ticks (e.g., 10ms = 10 ticks if timer is 1ms)
    time_quantum: u64,

    /// Ticks remaining in current thread's quantum
    quantum_remaining: u64,
}
```

### Step 2.2: Add Preemption Control API

```rust
impl Scheduler {
    /// Enable preemptive multitasking
    pub fn enable_preemption(&mut self, quantum_ms: u64) {
        self.preemption_enabled = true;
        self.time_quantum = quantum_ms;
        self.quantum_remaining = quantum_ms;
    }

    /// Disable preemptive multitasking (return to cooperative)
    pub fn disable_preemption(&mut self) {
        self.preemption_enabled = false;
    }

    /// Check if this thread's quantum has expired
    pub fn should_preempt(&mut self) -> bool {
        if !self.preemption_enabled {
            return false;
        }

        if self.quantum_remaining == 0 {
            // Quantum expired, reset for next thread
            self.quantum_remaining = self.time_quantum;
            return true;
        }

        false
    }

    /// Decrement the current thread's quantum
    pub fn tick_quantum(&mut self) {
        if self.quantum_remaining > 0 {
            self.quantum_remaining -= 1;
        }
    }
}
```

### Step 2.3: Add Public API in mod.rs

```rust
/// Enable preemptive multitasking with given time quantum
pub fn enable_preemption(quantum_ms: u64) {
    without_interrupts(|| {
        unsafe { get_loom().lock().enable_preemption(quantum_ms) }
    });
}

/// Disable preemptive multitasking (return to cooperative)
pub fn disable_preemption() {
    without_interrupts(|| {
        unsafe { get_loom().lock().disable_preemption() }
    });
}
```

**Verification:**
- [ ] Preemption can be enabled/disabled
- [ ] Default is disabled (cooperative mode)
- [ ] API is interrupt-safe

---

## Phase 3: Timer-Driven Context Switching

**Goal:** Make timer interrupt trigger context switches

### Step 3.1: Understand Current Timer Handler

Current code ([idt.rs:40-56](../heartwood/src/attunement/idt.rs#L40-L56)):
```rust
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::attunement::timer::tick();

    // REMOVED: yield_now() causes issues
    // TODO: Implement preemptive multitasking

    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }
}
```

### Step 3.2: Add Preemption Check to Timer Handler

**Modify timer handler:**
```rust
extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Increment tick counter
    crate::attunement::timer::tick();

    // Check if preemption is enabled and quantum expired
    let should_switch = unsafe {
        let mut loom = crate::loom_of_fate::get_loom().lock();
        loom.tick_quantum();
        loom.should_preempt()
    };

    // If quantum expired, trigger context switch
    if should_switch {
        crate::loom_of_fate::preemptive_yield();
    }

    // Send End of Interrupt
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }
}
```

### Step 3.3: Implement Preemptive Yield

**Add to loom_of_fate/mod.rs:**
```rust
/// Preemptive yield - called from timer interrupt
///
/// This is different from cooperative yield_now() because:
/// - We're already in an interrupt context
/// - We must be extremely careful with locks
/// - Stack frame is different (interrupt stack frame)
pub fn preemptive_yield() {
    // NOTE: Interrupts are already disabled in interrupt handler!

    unsafe {
        // Step 1: Lock scheduler and prepare for context switch
        let (should_switch, from_ctx_ptr, to_ctx_ptr) = {
            let mut loom = get_loom().lock();

            // Only yield if we have a current thread
            if loom.current_thread_id().is_none() {
                return;
            }

            // Prepare for context switch
            loom.prepare_yield()
        };

        // Step 2: If we should switch, do it
        if should_switch {
            // Context switch (lock is dropped)
            context::switch_context_cooperative(from_ctx_ptr, to_ctx_ptr);

            // After returning, update state
            let mut loom = get_loom().lock();
            loom.after_yield();
        }
    }
}
```

### Step 3.4: Handle Interrupt Stack Frame

**Challenge:** When switching from interrupt context, we need to handle the interrupt stack frame properly.

**Options:**
1. **Save/Restore Interrupt Stack:** Complex but complete
2. **Return from Interrupt, Then Switch:** Simpler but requires careful design
3. **Use Separate Interrupt Stacks:** Advanced but cleanest

**Recommended: Option 2 (Return First)**

Modify approach:
```rust
extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Increment tick counter
    crate::attunement::timer::tick();

    // Mark that we need to yield after returning from interrupt
    crate::loom_of_fate::request_preemptive_yield();

    // Send EOI and return from interrupt
    unsafe {
        super::PICS.lock().notify_end_of_interrupt(32);
    }

    // NOTE: After IRET, the CPU will check if a yield is pending
    // and perform it before continuing the interrupted thread
}
```

Then add a "yield pending" flag that gets checked after every interrupt return.

**Verification:**
- [ ] Timer interrupt increments quantum counter
- [ ] When quantum expires, context switch occurs
- [ ] System remains stable under preemption

---

## Phase 4: Testing and Debugging

**Goal:** Verify preemptive multitasking works correctly

### Test 4.1: Create CPU-Bound Test Thread

Create a thread that NEVER yields:
```rust
fn cpu_hog_thread() -> ! {
    let mut counter = 0u64;
    loop {
        counter += 1;

        // Print every million iterations (but DON'T yield!)
        if counter % 1_000_000 == 0 {
            crate::println!("[CPU Hog: {}]", counter);
        }

        // NO yield_now() call!
    }
}
```

**Expected behavior:**
- Without preemption: Other threads starve
- With preemption: Other threads still run

### Test 4.2: Create Lock Contention Test

Create threads that compete for locks:
```rust
static TEST_LOCK: InterruptSafeLock<u64> = InterruptSafeLock::new(0);

fn lock_test_thread() -> ! {
    loop {
        {
            let mut data = TEST_LOCK.lock();
            *data += 1;

            // Simulate work while holding lock
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }

        // Yield occasionally
        if data % 100 == 0 {
            yield_now();
        }
    }
}
```

**Expected behavior:**
- No deadlocks
- Lock counter increments correctly
- All threads make progress

### Test 4.3: Stress Test

Run system for extended period:
- Multiple threads
- Heavy allocation/deallocation
- Lock contention
- I/O operations

**Monitor for:**
- Deadlocks (system freezes)
- Memory corruption (crashes, panics)
- Lost interrupts (keyboard stops working)
- Stack corruption (garbage output)

### Test 4.4: Gradual Rollout

**Strategy:**
1. Start with preemption DISABLED (cooperative mode)
2. Enable preemption with LONG quantum (100ms)
3. Gradually reduce quantum (50ms, 20ms, 10ms)
4. Monitor stability at each step

**Verification:**
- [ ] CPU-bound threads are preempted
- [ ] No deadlocks under lock contention
- [ ] System stable for 1+ minute
- [ ] Keyboard still responsive

---

## Phase 5: Optimization and Tuning

**Goal:** Fine-tune preemptive scheduling for best performance

### Optimization 5.1: Dynamic Quantum Adjustment

Adjust quantum based on system load:
```rust
impl Scheduler {
    fn adjust_quantum(&mut self) {
        // If high harmony, allow longer quantums (less switching)
        if self.latest_metrics.system_harmony > 0.8 {
            self.time_quantum = 20; // 20ms
        }
        // If low harmony, use shorter quantums (more fair)
        else if self.latest_metrics.system_harmony < 0.5 {
            self.time_quantum = 5; // 5ms
        }
        // Normal: 10ms
        else {
            self.time_quantum = 10;
        }
    }
}
```

### Optimization 5.2: Priority-Based Preemption

High-priority threads can preempt low-priority ones:
```rust
impl Scheduler {
    fn should_preempt(&mut self) -> bool {
        if !self.preemption_enabled {
            return false;
        }

        // Quantum expired?
        if self.quantum_remaining == 0 {
            return true;
        }

        // High-priority thread waiting?
        if self.has_high_priority_ready() {
            let current_priority = self.get_current_priority();
            if current_priority < ThreadPriority::High {
                return true; // Preempt for higher priority
            }
        }

        false
    }
}
```

### Optimization 5.3: Interrupt Latency Reduction

Minimize time with interrupts disabled:
```rust
// BEFORE: Hold lock for entire operation
let result = {
    let lock = LOCK.lock(); // Interrupts disabled
    do_long_operation(&lock) // Interrupts disabled for ENTIRE operation
}; // Interrupts re-enabled

// AFTER: Copy data, release lock, then process
let data_copy = {
    let lock = LOCK.lock(); // Interrupts disabled
    lock.clone() // Quick copy
}; // Interrupts re-enabled

let result = do_long_operation(&data_copy); // Interrupts ENABLED during work
```

### Optimization 5.4: Context Switch Profiling

Track context switch overhead:
```rust
pub struct SchedulerStats {
    // ... existing fields ...

    /// Total time spent in context switches (in timer ticks)
    pub context_switch_overhead: u64,

    /// Average context switch time
    pub avg_switch_time: u64,
}
```

**Verification:**
- [ ] Context switches take < 100 microseconds
- [ ] Interrupt latency < 50 microseconds
- [ ] CPU efficiency > 95% (< 5% overhead)

---

## Rollback Plan

If preemptive multitasking causes issues:

### Immediate Rollback (Emergency)
1. Set `preemption_enabled = false` in scheduler
2. Rebuild and deploy
3. System returns to cooperative mode

### Conditional Compilation (Safe Rollback)
```rust
#[cfg(feature = "preemptive")]
const PREEMPTION_ENABLED: bool = true;

#[cfg(not(feature = "preemptive"))]
const PREEMPTION_ENABLED: bool = false;
```

Build with/without preemption:
```bash
# With preemption
cargo build --features preemptive

# Without preemption (safe mode)
cargo build
```

---

## Implementation Checklist

### Phase 1: Critical Sections (MUST complete first)
- [ ] Audit all mutexes in codebase
- [ ] Replace `spin::Mutex` with `InterruptSafeLock` in:
  - [ ] `mana_pool/mod.rs`
  - [ ] `loom_of_fate/mod.rs`
  - [ ] `nexus/mod.rs`
  - [ ] `vga_buffer.rs`
  - [ ] `eldarin.rs`
- [ ] Test that all locks work correctly
- [ ] Verify no deadlocks in current system

### Phase 2: Preemption Control
- [ ] Add preemption state to Scheduler
- [ ] Implement `enable_preemption()` / `disable_preemption()`
- [ ] Add quantum tracking
- [ ] Add public API in mod.rs
- [ ] Test enabling/disabling (should have no effect yet)

### Phase 3: Timer Integration
- [ ] Modify timer interrupt handler
- [ ] Implement `preemptive_yield()`
- [ ] Handle interrupt stack frame correctly
- [ ] Test with LONG quantum (100ms) first
- [ ] Gradually reduce quantum

### Phase 4: Testing
- [ ] Create CPU-bound test thread
- [ ] Test lock contention
- [ ] Stress test for 5+ minutes
- [ ] Verify keyboard still works
- [ ] Test stats display works
- [ ] Test memory allocation/deallocation

### Phase 5: Optimization
- [ ] Dynamic quantum adjustment
- [ ] Priority-based preemption
- [ ] Interrupt latency reduction
- [ ] Context switch profiling

---

## Success Criteria

Preemptive multitasking is considered successful when:

1. **Correctness:**
   - [ ] CPU-bound threads are preempted automatically
   - [ ] All threads make forward progress
   - [ ] No deadlocks occur

2. **Stability:**
   - [ ] System runs for 10+ minutes without crash
   - [ ] No memory corruption
   - [ ] No stack corruption

3. **Responsiveness:**
   - [ ] Keyboard input works during CPU-heavy load
   - [ ] UI remains responsive
   - [ ] Stats display updates correctly

4. **Performance:**
   - [ ] Context switch overhead < 5%
   - [ ] Interrupt latency < 50μs
   - [ ] System harmony maintained > 0.7

---

## Risk Assessment

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| Deadlocks from preemption | HIGH | MEDIUM | Use interrupt-safe locks everywhere |
| Stack corruption | HIGH | LOW | Disable interrupts during context switch |
| Drop implementation issues | MEDIUM | MEDIUM | Audit all Drop impls, make interrupt-safe |
| Performance degradation | LOW | LOW | Profile and optimize |
| Keyboard stops working | MEDIUM | LOW | Test interrupt handling thoroughly |

---

## Timeline Estimate

**Conservative estimate (with testing):**
- Phase 1: 2-3 days (critical, must be thorough)
- Phase 2: 1 day
- Phase 3: 2-3 days (complex, requires debugging)
- Phase 4: 2-3 days (extensive testing)
- Phase 5: 1-2 days (optional optimization)

**Total: 8-12 days**

**Aggressive estimate (experienced developer):**
- Phase 1: 1 day
- Phase 2: 4 hours
- Phase 3: 1-2 days
- Phase 4: 1 day
- Phase 5: 1 day

**Total: 4-5 days**

---

## References

### Technical Resources
- [OSDev Wiki: Interrupt Service Routines](https://wiki.osdev.org/Interrupt_Service_Routines)
- [OSDev Wiki: Context Switching](https://wiki.osdev.org/Context_Switching)
- x86_64 interrupt handling and stack frames
- [Rust Atomics and Locks](https://marabos.nl/atomics/) - Mara Bos

### Similar OS Implementations
- Linux: Completely Fair Scheduler (CFS)
- Redox OS: Rust-based preemptive scheduling
- [Writing an OS in Rust](https://os.phil-opp.com/) - Philipp Oppermann

### AethelOS-Specific
- [PRODUCTION_READINESS_PLAN.md](PRODUCTION_READINESS_PLAN.md) - Overall roadmap
- [heartwood/src/attunement/idt.rs](../heartwood/src/attunement/idt.rs) - Current timer handler
- [heartwood/src/loom_of_fate/scheduler.rs](../heartwood/src/loom_of_fate/scheduler.rs) - Scheduler implementation

---

## Notes

- **Start Simple:** Begin with long quantum (100ms), reduce gradually
- **Test Often:** After each phase, test thoroughly before proceeding
- **Have Rollback Ready:** Keep cooperative mode as fallback
- **Document Issues:** Keep log of bugs found and how they were fixed
- **Measure Everything:** Profile context switch times, interrupt latency
- **Maintain Philosophy:** Even with preemption, preserve AethelOS's harmony-based approach
