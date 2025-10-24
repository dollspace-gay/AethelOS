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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoomError {
    OutOfThreads,
    ThreadNotFound,
    InvalidPriority,
    StackAllocationFailed,
}
