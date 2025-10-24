//! Thread definitions - The Threads of Fate

use super::context::ThreadContext;

/// A unique identifier for a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(pub u64);

/// The state of a thread in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// The thread is actively running
    Weaving,

    /// The thread is idle, waiting for work
    Resting,

    /// The thread is blocked or has encountered an error
    Tangled,

    /// The thread is in the process of exiting
    Fading,
}

/// Priority levels for threads (used in harmony calculation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPriority {
    Critical = 0,  // System-critical threads
    High = 1,      // Important user-facing threads
    Normal = 2,    // Standard threads
    Low = 3,       // Background threads
    Idle = 4,      // Lowest priority
}

/// A thread of fate in the Loom
pub struct Thread {
    pub(crate) id: ThreadId,
    pub(crate) state: ThreadState,
    pub(crate) priority: ThreadPriority,
    pub(crate) entry_point: fn() -> !,

    // CPU state (for context switching)
    pub(crate) context: ThreadContext,
    pub(crate) stack_bottom: u64,
    pub(crate) stack_top: u64,

    // Harmony tracking
    pub(crate) resource_usage: ResourceUsage,
    pub(crate) harmony_score: f32,

    // Execution context
    pub(crate) time_slices_used: u64,
    pub(crate) yields: u64,
    pub(crate) last_run_time: u64,
}

impl Thread {
    /// Create a new thread with allocated stack
    ///
    /// # Arguments
    /// * `id` - Unique thread identifier
    /// * `entry_point` - Function where thread begins execution
    /// * `priority` - Thread priority level
    /// * `stack_bottom` - Low address of thread's stack
    /// * `stack_top` - High address of thread's stack
    pub fn new(
        id: ThreadId,
        entry_point: fn() -> !,
        priority: ThreadPriority,
        stack_bottom: u64,
        stack_top: u64,
    ) -> Self {
        // Create initial context for this thread
        let context = ThreadContext::new(entry_point as u64, stack_top);

        Self {
            id,
            state: ThreadState::Resting,
            priority,
            entry_point,
            context,
            stack_bottom,
            stack_top,
            resource_usage: ResourceUsage::default(),
            harmony_score: 1.0, // Start in perfect harmony
            time_slices_used: 0,
            yields: 0,
            last_run_time: 0,
        }
    }

    /// Get a mutable reference to the thread's context
    pub fn context_mut(&mut self) -> &mut ThreadContext {
        &mut self.context
    }

    /// Get a reference to the thread's context
    pub fn context(&self) -> &ThreadContext {
        &self.context
    }

    pub fn id(&self) -> ThreadId {
        self.id
    }

    pub fn state(&self) -> ThreadState {
        self.state
    }

    pub fn set_state(&mut self, state: ThreadState) {
        self.state = state;
    }

    pub fn priority(&self) -> ThreadPriority {
        self.priority
    }

    pub fn harmony_score(&self) -> f32 {
        self.harmony_score
    }

    pub fn set_harmony_score(&mut self, score: f32) {
        self.harmony_score = score.clamp(0.0, 1.0);
    }

    /// Record that this thread used a time slice
    pub fn record_time_slice(&mut self) {
        self.time_slices_used += 1;
    }

    /// Record that this thread yielded
    pub fn record_yield(&mut self) {
        self.yields += 1;
    }

    /// Check if this thread is exhibiting parasitic behavior
    pub fn is_parasite(&self) -> bool {
        self.harmony_score < 0.3
    }

    /// Get the thread's resource usage
    pub fn resource_usage(&self) -> &ResourceUsage {
        &self.resource_usage
    }

    /// Update resource usage statistics
    pub fn update_resource_usage(&mut self, usage: ResourceUsage) {
        self.resource_usage = usage;
    }
}

/// Tracks a thread's resource consumption
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceUsage {
    pub cpu_time: u64,
    pub memory_allocated: usize,
    pub messages_sent: u64,
}
