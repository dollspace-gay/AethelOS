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

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref LOOM: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

/// Initialize the Loom of Fate
pub fn init() {
    let _ = LOOM.lock();
}

/// Spawn a new thread
pub fn spawn(entry_point: fn() -> !, priority: ThreadPriority) -> Result<ThreadId, LoomError> {
    LOOM.lock().spawn(entry_point, priority)
}

/// Yield the current thread
pub fn yield_now() {
    LOOM.lock().yield_current();
}

/// Get the current thread ID
pub fn current_thread() -> Option<ThreadId> {
    LOOM.lock().current_thread_id()
}

/// Get scheduler statistics
pub fn stats() -> SchedulerStats {
    LOOM.lock().stats()
}

/// Start the system threads and begin multitasking
///
/// This function spawns the initial system threads and then yields,
/// allowing the scheduler to begin running threads.
///
/// The spawned threads are:
/// - Idle thread (Priority::Idle) - runs when nothing else can
/// - Keyboard thread (Priority::High) - processes keyboard input
/// - Shell thread (Priority::Normal) - interactive shell
///
/// # Note
/// This function will return after spawning threads, allowing the
/// caller to continue initialization or enter its own loop.
pub fn start_system_threads() -> Result<(), LoomError> {
    crate::println!("◈ Starting Loom of Fate threads...");

    // Spawn the idle thread (lowest priority)
    spawn(system_threads::idle_thread, ThreadPriority::Idle)?;

    // Spawn the keyboard handler thread (high priority - user input is important)
    spawn(system_threads::keyboard_thread, ThreadPriority::High)?;

    // Spawn the shell thread (normal priority)
    spawn(system_threads::shell_thread, ThreadPriority::Normal)?;

    crate::println!("◈ System threads ready. Harmony: {:.2}", stats().system_harmony);
    crate::println!();

    Ok(())
}

/// Begin multitasking by yielding to the scheduler
///
/// This is typically called after start_system_threads() to actually
/// start executing threads.
pub fn begin_weaving() -> ! {
    crate::println!("◈ The Loom begins to weave...");
    crate::println!();

    // Yield to start the first thread
    // This will never return as we'll be context-switched away
    loop {
        yield_now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoomError {
    OutOfThreads,
    ThreadNotFound,
    InvalidPriority,
    StackAllocationFailed,
}
