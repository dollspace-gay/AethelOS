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
pub mod vessel;
pub mod harbor;
pub mod syscalls;
pub mod elf_loader;

pub use scheduler::{Scheduler, SchedulerStats};
pub use thread::{Thread, ThreadId, ThreadState, ThreadPriority, ThreadType};
pub use harmony::{HarmonyAnalyzer, HarmonyMetrics};
pub use vessel::{Vessel, VesselId, VesselState};
pub use harbor::{Harbor, HarborStats};
pub use syscalls::{dispatch_syscall, SyscallResult, SyscallError};
pub use elf_loader::{load_elf, LoadedElf, ElfError};
pub use context::ThreadContext;

use crate::mana_pool::InterruptSafeLock;
use core::mem::MaybeUninit;
use alloc::boxed::Box;

// Manual static initialization using MaybeUninit
// LOOM stores a Box<Scheduler> - a small pointer to heap-allocated Scheduler
// Using InterruptSafeLock to prevent deadlocks during preemptive multitasking
static mut LOOM: MaybeUninit<InterruptSafeLock<Box<Scheduler>>> = MaybeUninit::uninit();
static mut LOOM_INITIALIZED: bool = false;

// HARBOR stores the process table (registry of all Vessels)
// Using InterruptSafeLock to prevent deadlocks during preemptive multitasking
static mut HARBOR: MaybeUninit<InterruptSafeLock<Harbor>> = MaybeUninit::uninit();
static mut HARBOR_INITIALIZED: bool = false;

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
    // CRITICAL: Disable interrupts during LOOM initialization to prevent deadlock!
    // Keyboard interrupts can fire and try to lock LOOM while we're initializing it.
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
        for &byte in b"[LOOM INIT] Interrupts disabled\n".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    unsafe {
        serial_out(b'1'); // Before init

        serial_out(b'A'); // About to call Scheduler::new_boxed
        let scheduler_on_heap = Scheduler::new_boxed();
        serial_out(b'B'); // Scheduler::new_boxed returned

        // Create an interrupt-safe lock around the small Box pointer
        serial_out(b'C'); // About to create InterruptSafeLock
        let lock = InterruptSafeLock::new(scheduler_on_heap, "LOOM");
        serial_out(b'D'); // InterruptSafeLock created

        // Write the small InterruptSafeLock<Box<Scheduler>> to static
        core::ptr::write(core::ptr::addr_of_mut!(LOOM).cast(), lock);
        serial_out(b'y'); // Written to MaybeUninit

        LOOM_INITIALIZED = true;
        serial_out(b'3'); // Marked as initialized
    }

    // Initialize Harbor (process table)
    unsafe {
        serial_out(b'H'); // Harbor init start
        let harbor = Harbor::new();
        let harbor_lock = InterruptSafeLock::new(harbor, "HARBOR");
        core::ptr::write(core::ptr::addr_of_mut!(HARBOR).cast(), harbor_lock);
        HARBOR_INITIALIZED = true;
        serial_out(b'h'); // Harbor initialized

        for &byte in b"[HARBOR INIT] Harbor initialized\n".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    // Now create the system threads (but don't start them yet)
    crate::println!("◈ Forging the system threads...");

    // Spawn the idle thread (lowest priority - will be first to run)
    // CRITICAL: Do NOT use .expect() here! It can panic with formatting,
    // which might try to lock LOOM again, causing deadlock.
    match spawn(system_threads::idle_thread, ThreadPriority::Idle) {
        Ok(_) => {},
        Err(_) => {
            unsafe {
                let msg = b"\n[FATAL] Failed to spawn idle thread!\n";
                for &byte in msg.iter() {
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") byte,
                        options(nomem, nostack, preserves_flags)
                    );
                }
                loop {
                    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                }
            }
        }
    }

    // Spawn the keyboard handler thread
    match spawn(system_threads::keyboard_thread, ThreadPriority::High) {
        Ok(_) => {},
        Err(_) => {
            unsafe {
                let msg = b"\n[FATAL] Failed to spawn keyboard thread!\n";
                for &byte in msg.iter() {
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") byte,
                        options(nomem, nostack, preserves_flags)
                    );
                }
                loop {
                    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                }
            }
        }
    }

    // Spawn the shell thread
    match spawn(system_threads::shell_thread, ThreadPriority::Normal) {
        Ok(_) => {},
        Err(_) => {
            unsafe {
                let msg = b"\n[FATAL] Failed to spawn shell thread!\n";
                for &byte in msg.iter() {
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3f8u16,
                        in("al") byte,
                        options(nomem, nostack, preserves_flags)
                    );
                }
                loop {
                    core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                }
            }
        }
    }

    // Debug: Verify thread contexts are correct
    // CRITICAL: Collect data FIRST, release lock, THEN print
    // to avoid deadlock if println tries to lock LOOM
    unsafe {
        let mut thread_info = [(0u64, 0u64, 0u64); 3];
        {
            let loom = get_loom().lock();
            for tid_val in 1u64..=3u64 {
                if let Some(ctx) = loom.get_thread_context(ThreadId(tid_val)) {
                    thread_info[(tid_val - 1) as usize] = (tid_val, (*ctx).rsp, (*ctx).rip);
                }
            }
            // Lock released here
        }

        // Now print AFTER releasing the lock
        for (tid, rsp, rip) in thread_info.iter() {
            if *tid != 0 {
                crate::println!("  Thread {}: rsp={:#x}, rip={:#x}", tid, rsp, rip);
            }
        }
    }

    crate::println!("◈ System threads forged. Ready for the Great Hand-Off.");

    // NOTE: Interrupts remain disabled here. They will be enabled later by
    // attunement::init() after the IDT is properly set up.
    unsafe {
        for &byte in b"[LOOM INIT] Complete (interrupts still disabled)\n".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

/// Get reference to LOOM (assumes initialized)
///
/// # Safety
/// LOOM must be initialized before calling this function
pub unsafe fn get_loom() -> &'static InterruptSafeLock<Box<Scheduler>> {
    &*core::ptr::addr_of!(LOOM).cast::<InterruptSafeLock<Box<Scheduler>>>()
}

/// Get reference to Harbor (process table)
///
/// # Safety
/// Harbor must be initialized before calling this function
pub fn get_harbor() -> &'static InterruptSafeLock<Harbor> {
    unsafe {
        if !HARBOR_INITIALIZED {
            panic!("Harbor not initialized!");
        }
        &*core::ptr::addr_of!(HARBOR).cast::<InterruptSafeLock<Harbor>>()
    }
}

/// Spawn a new thread
pub fn spawn(entry_point: fn() -> !, priority: ThreadPriority) -> Result<ThreadId, LoomError> {
    unsafe { get_loom().lock().spawn(entry_point, priority) }
}

/// Create a user-mode thread for a Vessel
///
/// This creates a thread that will execute in ring 3 (user mode) within
/// the specified Vessel's address space.
///
/// # Arguments
/// * `vessel_id` - The Vessel this thread belongs to
/// * `entry_point` - User space entry point address
/// * `user_stack_top` - Top of user stack
/// * `priority` - Thread priority
///
/// # Returns
/// ThreadId of the created thread
///
/// # Note
/// Creates a user-mode thread within a Vessel (Ring 3 execution).
pub fn create_user_thread(
    vessel_id: VesselId,
    entry_point: u64,
    user_stack_top: u64,
    priority: ThreadPriority,
) -> Result<ThreadId, LoomError> {
    without_interrupts(|| {
        let mut loom = unsafe { get_loom().lock() };

        // Get Vessel info (page table address)
        let harbor = get_harbor().lock();
        let vessel = harbor.find_vessel(vessel_id)
            .ok_or(LoomError::VesselNotFound)?;
        let page_table_phys = vessel.page_table_phys();
        drop(harbor);

        // Generate thread ID
        let thread_id = ThreadId(loom.next_thread_id);
        loom.next_thread_id += 1;

        // Create user-mode context (Ring 3 with user segments)
        let context = ThreadContext::new_user_mode(
            entry_point,
            user_stack_top,
            page_table_phys,
        );

        // Create thread with pre-initialized context
        let thread = Thread::new_with_context(
            thread_id,
            context,
            priority,
            ThreadType::User,  // This is a Ring 3 user thread
            Some(vessel_id),
        );

        // Add to thread list and ready queue
        loom.threads.push(thread);
        loom.ready_queue.push_back(thread_id);

        crate::serial_println!("[LOOM] Created user thread {} for Vessel {} at entry {:#x}",
                               thread_id.0, vessel_id.0, entry_point);

        Ok(thread_id)
    })
}

/// Create a Ring 1 service thread for a Grove (privileged service)
///
/// # Arguments
/// * `vessel_id` - The Vessel (service process) this thread belongs to
/// * `entry_point` - Virtual address where service execution begins
/// * `service_stack_top` - Top of the service's stack (16-byte aligned)
/// * `priority` - Thread priority level
///
/// # Returns
/// * `Ok(ThreadId)` - The ID of the newly created service thread
/// * `Err(LoomError)` - If thread creation fails
pub fn create_service_thread(
    vessel_id: VesselId,
    entry_point: u64,
    service_stack_top: u64,
    priority: ThreadPriority,
) -> Result<ThreadId, LoomError> {
    without_interrupts(|| {
        let mut loom = unsafe { get_loom().lock() };

        // Get Vessel info (page table address)
        let harbor = get_harbor().lock();
        let vessel = harbor.find_vessel(vessel_id)
            .ok_or(LoomError::VesselNotFound)?;
        let page_table_phys = vessel.page_table_phys();
        drop(harbor);

        // Generate thread ID
        let thread_id = ThreadId(loom.next_thread_id);
        loom.next_thread_id += 1;

        // Create Ring 1 service context
        let context = ThreadContext::new_service_mode(
            entry_point,
            service_stack_top,
            page_table_phys,
        );

        // Create thread with pre-initialized context
        let thread = Thread::new_with_context(
            thread_id,
            context,
            priority,
            ThreadType::Service,  // This is a Ring 1 service thread
            Some(vessel_id),
        );

        // Add to thread list and ready queue
        loom.threads.push(thread);
        loom.ready_queue.push_back(thread_id);

        crate::serial_println!("[LOOM] Created service thread {} for Vessel {} at entry {:#x}",
                              thread_id.0, vessel_id.0, entry_point);

        Ok(thread_id)
    })
}

/// Terminate a thread by ID
///
/// Marks the thread as Fading so it won't be scheduled again.
/// The thread's resources will be cleaned up by the scheduler.
///
/// # Arguments
/// * `thread_id` - The ID of the thread to terminate
///
/// # Returns
/// * `Ok(())` - Thread marked as Fading
/// * `Err(LoomError)` - If thread not found
pub fn terminate_thread(thread_id: ThreadId) -> Result<(), LoomError> {
    without_interrupts(|| {
        unsafe {
            let mut loom = get_loom().lock();

            // Find the thread and mark it as Fading
            if let Some(thread) = loom.threads.iter_mut().find(|t| t.id() == thread_id) {
                thread.set_state(ThreadState::Fading);

                // Remove from ready queue if present
                loom.ready_queue.retain(|&tid| tid != thread_id);

                crate::serial_println!("[LOOM] Terminated thread {:?}", thread_id);
                Ok(())
            } else {
                Err(LoomError::ThreadNotFound)
            }
        }
    })
}

/// Execute a closure with interrupts disabled
/// This prevents deadlocks when acquiring locks that might be used in interrupt handlers
pub(crate) fn without_interrupts<F, R>(f: F) -> R
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
    // DEBUG: Mark function entry
    unsafe {
        for &byte in b"[FUNC:yield_now]".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    // CRITICAL: Disable interrupts while holding the scheduler lock
    // to prevent deadlock when timer interrupt fires during context switch.
    // The lock MUST be held across the context switch.
    without_interrupts(|| {
        unsafe {
            // Step 1: Lock scheduler
            let mut loom = get_loom().lock();

            // Only yield if we actually have a current thread
            if loom.current_thread_id().is_none() {
                // No thread, just return. Lock will be released.
                return;
            }

            // Step 2: Prepare for context switch
            let (should_switch, from_ctx_ptr, to_ctx_ptr, new_kernel_stack) = loom.prepare_yield();

            crate::serial_println!("[YIELD] After prepare_yield: should_switch={}, new_kernel_stack={:?}",
                                   should_switch, new_kernel_stack);

            // Step 3: If we should switch, do it
            if should_switch {
                crate::serial_println!("[YIELD] Inside should_switch block");

                // Check if we need IRETQ-based switching (for privilege level changes)
                // We need IRETQ if:
                // 1. The TARGET thread requires a privilege change (has kernel_stack), OR
                // 2. The SOURCE thread is not at Ring 0 (CS != 0x08)
                let target_needs_iretq = new_kernel_stack.is_some();
                let source_cs = unsafe { (*from_ctx_ptr).cs };
                let source_needs_iretq = source_cs != 0x08;
                let needs_iretq = target_needs_iretq || source_needs_iretq;

                crate::serial_println!("[YIELD] target_needs_iretq={}, source_cs={:#x}, source_needs_iretq={}, needs_iretq={}",
                                       target_needs_iretq, source_cs, source_needs_iretq, needs_iretq);

                // Update TSS.rsp[0] if switching to a user-mode thread
                if let Some(kernel_stack) = new_kernel_stack {
                    crate::serial_println!("[YIELD] About to set_kernel_stack({:#x})", kernel_stack);
                    crate::attunement::set_kernel_stack(kernel_stack);
                    crate::serial_println!("[YIELD] ✓ set_kernel_stack completed");
                }

                // CRITICAL: Release the lock BEFORE the context switch!
                // We MUST do this because the new thread might be starting from its
                // entry point (not resuming from yield_now), and wouldn't know to
                // release the lock, causing a deadlock.
                drop(loom);

                // Now do the context switch WITHOUT holding the lock
                // Interrupts are still disabled by without_interrupts(), so this is safe

                // CRITICAL: Use IRETQ-based switch if ANY privilege level change is needed!
                // switch_context_cooperative uses RET which cannot change privilege levels
                // switch_context uses IRETQ which can transition between any rings
                if needs_iretq {
                    // DEBUG: Check RSP before calling switch_context
                    unsafe {
                        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'U', options(nomem, nostack, preserves_flags));
                        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'S', options(nomem, nostack, preserves_flags));
                        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'R', options(nomem, nostack, preserves_flags));

                        // Check current RSP - if it's a user address, we'll fault on CALL
                        let current_rsp: u64;
                        core::arch::asm!("mov {}, rsp", out(reg) current_rsp, options(nomem, nostack, preserves_flags));

                        // Check if RSP is in user space (< 0x8000000000000000)
                        if current_rsp < 0x8000000000000000 {
                            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'!', options(nomem, nostack, preserves_flags));
                            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'U', options(nomem, nostack, preserves_flags));
                            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'S', options(nomem, nostack, preserves_flags));
                            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'R', options(nomem, nostack, preserves_flags));
                        } else {
                            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b'K', options(nomem, nostack, preserves_flags));
                        }
                    }
                    context::switch_context(from_ctx_ptr, to_ctx_ptr);
                    crate::serial_println!("[YIELD] ✓ Returned from switch_context");
                } else {
                    context::switch_context_cooperative(from_ctx_ptr, to_ctx_ptr);
                }

                // --- WE ARE NOW THE NEW THREAD ---
                // The lock was released before the switch, so we don't hold it
                // No cleanup needed
            }

            // Step 4: The `without_interrupts` guard drops here,
            // re-enabling interrupts safely.
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
    let (should_switch, from_ctx_ptr, to_ctx_ptr, new_kernel_stack) = {
        let mut loom = get_loom().lock();

        // Only yield if we actually have a current thread
        if loom.current_thread_id().is_none() {
            // No current thread - this shouldn't happen, but handle it gracefully
            // Release lock and return via IRETQ
            drop(loom);
            core::arch::asm!(
                "iretq",
                options(noreturn)
            );
        }

        // Prepare for context switch and get pointers
        let result = loom.prepare_yield();
        // Lock automatically drops here
        result
    };

    // Update TSS.rsp[0] if switching to a user-mode thread
    if let Some(kernel_stack) = new_kernel_stack {
        crate::attunement::set_kernel_stack(kernel_stack);
    }

    // Step 2: If we should switch, save current context and switch
    // NOTE: Lock is released. This is safe because we're in interrupt context
    // with interrupts disabled - no other interrupt can interfere
    if should_switch {
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
    // DEBUG: Mark function entry
    for &byte in b"[FUNC:get_idle_ctx]".iter() {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") byte,
            options(nomem, nostack, preserves_flags)
        );
    }

    let loom = get_loom().lock();

    // The idle thread should be thread ID 1 (first spawned)
    let idle_thread_id = ThreadId(1);

    // CRITICAL: Do NOT use .expect() here! It can panic with formatting,
    // which might try to lock LOOM again, causing deadlock.
    match loom.get_thread_context(idle_thread_id) {
        Some(ctx) => ctx,
        None => {
            // Idle thread not found - output error and halt WITHOUT panicking
            let msg = b"\n[FATAL] Idle thread not found in LOOM!\n";
            for &byte in msg.iter() {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") 0x3f8u16,
                    in("al") byte,
                    options(nomem, nostack, preserves_flags)
                );
            }
            loop {
                core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
            }
        }
    }
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
    VesselNotFound,
}
