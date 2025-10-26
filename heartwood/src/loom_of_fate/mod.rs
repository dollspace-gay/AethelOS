//! # The Loom of Fate
//!
//! The harmony-based scheduler of AethelOS.
//! The Loom does not preempt; it negotiates.
//! It does not kill greedy processes; it soothes them.
//!
//! ## Philosophy
//! Every thread is a thread of fate, weaving its purpose into the tapestry
//! of the system. The Loom's role is to maintain harmony, ensuring that
//! no thread dominates while all threads progress toward their destiny.
//!
//! ## Architecture
//! - Cooperative scheduling with implicit yielding
//! - Thread states: Weaving, Resting, Tangled, Fading
//! - Resource negotiation based on system-wide harmony
//! - Parasite detection and throttling (not killing)

pub mod context;
pub mod scheduler;
pub mod stack;
pub mod system_threads;
pub mod thread;
pub mod harmony;

pub use scheduler::{Scheduler, SchedulerStats};
pub use thread::{Thread, ThreadId, ThreadState, ThreadPriority};
pub use harmony::{HarmonyAnalyzer, HarmonyMetrics};

use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;
use alloc::boxed::Box;

// Manual static initialization using MaybeUninit
// LOOM stores a Box<Scheduler> - a small pointer to heap-allocated Scheduler
// Using InterruptSafeLock to prevent deadlocks during preemptive multitasking
static mut LOOM: MaybeUninit<InterruptSafeLock<Box<Scheduler>>> = MaybeUninit::uninit();
static mut LOOM_INITIALIZED: bool = false;

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

/// Initialize the Loom of Fate and create system threads
///
/// This creates the scheduler and spawns the three system threads:
/// - Idle thread (will be the first to run)
/// - Keyboard thread
/// - Shell thread
///
/// Note: This does NOT start the threads or enable interrupts.
/// That happens during the Great Hand-Off.
pub fn init() {
    unsafe {
        serial_out(b'1'); // Before init

        serial_out(b'A'); // About to call Scheduler::new_boxed
        let scheduler_on_heap = Scheduler::new_boxed();
        serial_out(b'B'); // Scheduler::new_boxed returned

        // Create an interrupt-safe lock around the small Box pointer
        serial_out(b'C'); // About to create InterruptSafeLock
        let lock = InterruptSafeLock::new(scheduler_on_heap);
        serial_out(b'D'); // InterruptSafeLock created

        // Write the small InterruptSafeLock<Box<Scheduler>> to static
        core::ptr::write(core::ptr::addr_of_mut!(LOOM).cast(), lock);
        serial_out(b'y'); // Written to MaybeUninit

        LOOM_INITIALIZED = true;
        serial_out(b'3'); // Marked as initialized
    }

    // Now create the system threads (but don't start them yet)
    crate::println!("◈ Forging the system threads...");

    // Spawn the idle thread (lowest priority - will be first to run)
    spawn(system_threads::idle_thread, ThreadPriority::Idle)
        .expect("Failed to spawn idle thread");

    // Spawn the keyboard handler thread
    spawn(system_threads::keyboard_thread, ThreadPriority::High)
        .expect("Failed to spawn keyboard thread");

    // Spawn the shell thread
    spawn(system_threads::shell_thread, ThreadPriority::Normal)
        .expect("Failed to spawn shell thread");

    // Debug: Verify thread contexts are correct
    unsafe {
        let loom = get_loom().lock();
        for tid in 1..=3 {
            if let Some(ctx) = loom.get_thread_context(ThreadId(tid)) {
                crate::println!("  Thread {}: context@{:p}, rsp={:#x}, rip={:#x}",
                               tid, ctx, (*ctx).rsp, (*ctx).rip);
            }
        }
    }

    crate::println!("◈ System threads forged. Ready for the Great Hand-Off.");
}

/// Get reference to LOOM (assumes initialized)
///
/// # Safety
/// LOOM must be initialized before calling this function
pub unsafe fn get_loom() -> &'static InterruptSafeLock<Box<Scheduler>> {
    &*core::ptr::addr_of!(LOOM).cast::<InterruptSafeLock<Box<Scheduler>>>()
}

/// Spawn a new thread
pub fn spawn(entry_point: fn() -> !, priority: ThreadPriority) -> Result<ThreadId, LoomError> {
    unsafe { get_loom().lock().spawn(entry_point, priority) }
}

/// Execute a closure with interrupts disabled
/// This prevents deadlocks when acquiring locks that might be used in interrupt handlers
fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    // Check if interrupts are currently enabled
    let were_enabled: bool;
    unsafe {
        let flags: u64;
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) flags,
            options(nomem, preserves_flags)
        );
        were_enabled = (flags & 0x200) != 0;
    }

    // Disable interrupts
    if were_enabled {
        unsafe {
            core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
        }
    }

    // Execute the closure
    let result = f();

    // Re-enable interrupts if they were enabled before
    if were_enabled {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
        }
    }

    result
}

/// Yield the current thread
pub fn yield_now() {
    // CRITICAL: Disable interrupts while holding the scheduler lock
    // to prevent deadlock when timer interrupt fires during context switch
    without_interrupts(|| {
        unsafe {
            // Step 1: Lock scheduler and prepare for context switch
            let (should_switch, from_ctx_ptr, to_ctx_ptr) = {
                let mut loom = get_loom().lock();

                // Only yield if we actually have a current thread
                if loom.current_thread_id().is_none() {
                    return;
                }

                // Prepare for context switch and get pointers
                loom.prepare_yield()
            };

            // Step 2: If we should switch, drop the lock and do the context switch
            if should_switch {
                // The lock is now dropped! (loom was dropped at end of block above)
                serial_out(b'L'); // Lock dropped!
                // Now we can safely context switch without holding any locks
                serial_out(b'C'); // About to context switch
                context::switch_context_cooperative(from_ctx_ptr, to_ctx_ptr);
                serial_out(b'R'); // Returned from context switch

                // When we return here, we're running as a different thread
                // Re-lock and update state
                let mut loom = get_loom().lock();
                loom.after_yield();
            }
        }
    });
}

/// Preemptive yield - called from timer interrupt when quantum expires
///
/// This is specifically designed for interrupt context and properly handles
/// the interrupt stack frame to avoid corruption.
///
/// # Arguments
/// * `interrupt_frame_ptr` - Pointer to the interrupt stack frame (RIP, CS, RFLAGS, RSP, SS)
///
/// # Safety
/// - Must only be called from timer interrupt handler
/// - Interrupts are already disabled in interrupt context
/// - All locks are interrupt-safe (Phase 1 complete)
/// - The interrupt_frame_ptr must point to the valid interrupt frame on stack
pub unsafe fn preemptive_yield(interrupt_frame_ptr: *const u64) -> ! {
    // We're already in interrupt context with interrupts disabled
    // No need for without_interrupts() wrapper

    // Step 1: Lock scheduler and prepare for context switch
    let (should_switch, from_ctx_ptr, to_ctx_ptr) = {
        let mut loom = get_loom().lock();

        // Only yield if we actually have a current thread
        if loom.current_thread_id().is_none() {
            // No current thread - this shouldn't happen, but handle it gracefully
            // We need to return via IRETQ, not return normally
            // Just restore the interrupted state
            drop(loom);
            core::arch::asm!(
                "iretq",
                options(noreturn)
            );
        }

        // Prepare for context switch and get pointers
        loom.prepare_yield()
    };

    // Step 2: If we should switch, save current context and switch
    if should_switch {
        // The lock is now dropped! (loom was dropped at end of block above)

        // Save the interrupted thread's context from the interrupt frame
        context::save_preempted_context(from_ctx_ptr as *mut context::ThreadContext, interrupt_frame_ptr);

        // Now restore the new thread's context and jump to it
        // This uses IRETQ and never returns
        context::restore_context(to_ctx_ptr);

        // UNREACHABLE - restore_context uses iretq and never returns
    } else {
        // No context switch needed - just return from interrupt normally
        // Use IRETQ to properly return from the interrupt
        core::arch::asm!(
            "iretq",
            options(noreturn)
        );
    }
}

/// Get the current thread ID
pub fn current_thread() -> Option<ThreadId> {
    without_interrupts(|| {
        unsafe { get_loom().lock().current_thread_id() }
    })
}

/// Get scheduler statistics
pub fn stats() -> SchedulerStats {
    without_interrupts(|| {
        unsafe { get_loom().lock().stats() }
    })
}

/// Thread debug information for security ward display
#[derive(Debug, Clone, Copy)]
pub struct ThreadDebugInfo {
    pub id: u64,
    pub stack_bottom: u64,
    pub stack_top: u64,
    pub stack_size: u64,
    pub state: ThreadState,
    pub priority: ThreadPriority,
}

/// Get debug information for all active threads
///
/// Returns detailed information about each thread's stack layout,
/// useful for verifying ASLR randomization and memory layout.
pub fn get_thread_debug_info() -> alloc::vec::Vec<ThreadDebugInfo> {
    without_interrupts(|| {
        unsafe {
            let loom = get_loom().lock();
            loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .map(|t| {
                    let stack_size = t.stack_top.saturating_sub(t.stack_bottom);
                    ThreadDebugInfo {
                        id: t.id.0,
                        stack_bottom: t.stack_bottom,
                        stack_top: t.stack_top,
                        stack_size,
                        state: t.state,
                        priority: t.priority,
                    }
                })
                .collect()
        }
    })
}

// === Preemptive Multitasking Control ===

/// Enable preemptive multitasking with the given time quantum
///
/// # Arguments
/// * `quantum_ms` - Time quantum in milliseconds (e.g., 10 = 10ms per thread)
///
/// When enabled, the timer interrupt will trigger context switches
/// after the quantum expires, even if the thread hasn't yielded.
///
/// # Example
/// ```
/// // Enable preemption with 10ms quantum
/// loom_of_fate::enable_preemption(10);
/// ```
pub fn enable_preemption(quantum_ms: u64) {
    without_interrupts(|| {
        unsafe { get_loom().lock().enable_preemption(quantum_ms) }
    });
}

/// Disable preemptive multitasking (return to cooperative mode)
///
/// Threads will only switch when they explicitly call yield_now().
pub fn disable_preemption() {
    without_interrupts(|| {
        unsafe { get_loom().lock().disable_preemption() }
    });
}

/// Check if preemption is currently enabled
pub fn is_preemption_enabled() -> bool {
    without_interrupts(|| {
        unsafe { get_loom().lock().is_preemption_enabled() }
    })
}

/// Get the current time quantum setting
pub fn get_time_quantum() -> u64 {
    without_interrupts(|| {
        unsafe { get_loom().lock().get_time_quantum() }
    })
}

/// Get a pointer to the idle thread's context for the Great Hand-Off
///
/// This is called once during bootstrap to get the entry point
/// for the first thread (idle thread).
///
/// # Safety
/// Returns a raw pointer to the idle thread's context structure
pub unsafe fn get_idle_thread_context() -> *const context::ThreadContext {
    let loom = get_loom().lock();

    // The idle thread should be thread ID 1 (first spawned)
    let idle_thread_id = ThreadId(1);

    loom.get_thread_context(idle_thread_id)
        .expect("Idle thread not found!")
}

/// Prepare for the Great Hand-Off by setting the idle thread as current
///
/// This MUST be called before context_switch_first to ensure the scheduler
/// knows which thread is running after the hand-off.
pub unsafe fn prepare_great_handoff() {
    let mut loom = get_loom().lock();
    let idle_thread_id = ThreadId(1);
    loom.prepare_handoff(idle_thread_id);
}

/// Begin multitasking - The Sacred First Weave (DEPRECATED)
///
/// This performs the one-time transition from bootstrap code to the first
/// real thread. Unlike normal context switches, this is a one-way journey.
///
/// This function never returns - control passes to the threading system forever.
pub fn begin_weaving() -> ! {
    crate::println!("◈ The Loom begins to weave...");
    crate::println!();

    // CRITICAL: Ensure interrupts are enabled before launching threads
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    // Perform the sacred first weave - a one-way journey from bootstrap to threads
    // This will never return - we hand over control to the Loom forever
    unsafe {
        get_loom().lock().start_weaving();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoomError {
    OutOfThreads,
    ThreadNotFound,
    InvalidPriority,
    StackAllocationFailed,
}
