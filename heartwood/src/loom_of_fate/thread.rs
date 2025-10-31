//! Thread definitions - The Threads of Fate

use super::context::ThreadContext;
use super::vessel::VesselId;

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

    /// Entry point function - kept for debugging/inspection
    #[allow(dead_code)]
    pub(crate) entry_point: fn() -> !,

    // CPU state (for context switching)
    pub(crate) context: ThreadContext,

    /// Stack boundaries - kept for future stack overflow detection
    #[allow(dead_code)]
    pub(crate) stack_bottom: u64,
    #[allow(dead_code)]
    pub(crate) stack_top: u64,

    /// The Weaver's Sigil - unique per-thread stack canary
    /// This secret value protects against stack buffer overflows
    /// It should NEVER be exposed to userspace
    pub(crate) sigil: u64,

    /// The Vessel (process) this thread belongs to
    /// None for kernel threads that don't belong to any Vessel
    pub(crate) vessel_id: Option<VesselId>,

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
    /// * `vessel_id` - VesselId this thread belongs to (None for kernel threads)
    pub fn new(
        id: ThreadId,
        entry_point: fn() -> !,
        priority: ThreadPriority,
        stack_bottom: u64,
        stack_top: u64,
        vessel_id: Option<VesselId>,
    ) -> Self {
        // Create initial context for this thread
        let context = ThreadContext::new(entry_point as u64, stack_top);

        // Generate unique Weaver's Sigil (stack canary) for this thread
        let sigil = Self::generate_sigil();

        // DEBUG: Verify context was created correctly
        crate::println!("  Thread::new - id={}, entry={:#x}, stack_top={:#x}, context.rsp={:#x}",
                       id.0, entry_point as u64, stack_top, context.rsp);
        crate::println!("  Weaver's Sigil: 0x{:016x}", sigil);

        Self {
            id,
            state: ThreadState::Resting,
            priority,
            entry_point,
            context,
            stack_bottom,
            stack_top,
            sigil,
            vessel_id,
            resource_usage: ResourceUsage::default(),
            harmony_score: 1.0, // Start in perfect harmony
            time_slices_used: 0,
            yields: 0,
            last_run_time: 0,
        }
    }

    /// Generate a unique Weaver's Sigil (stack canary) for this thread
    ///
    /// Uses ChaCha8 RNG seeded from hardware (RDTSC) to generate
    /// a cryptographically strong 64-bit random value.
    ///
    /// # Security
    /// This value MUST remain secret and never be exposed to userspace.
    fn generate_sigil() -> u64 {
        use crate::mana_pool::entropy::ChaCha8Rng;

        let mut rng = ChaCha8Rng::from_hardware_fast();
        let high = rng.next_u32() as u64;
        let low = rng.next_u32() as u64;
        (high << 32) | low
    }

    /// Create a thread with a pre-initialized context
    ///
    /// Used for user-mode threads where the context is set up specially
    /// for ring 3 execution.
    ///
    /// # Arguments
    /// * `id` - Unique thread identifier
    /// * `context` - Pre-initialized ThreadContext (for user mode)
    /// * `priority` - Thread priority level
    /// * `vessel_id` - VesselId this thread belongs to (Some for user threads)
    ///
    /// # Returns
    /// A new Thread with the given context
    pub fn new_with_context(
        id: ThreadId,
        context: ThreadContext,
        priority: ThreadPriority,
        vessel_id: Option<VesselId>,
    ) -> Self {
        // Generate unique Weaver's Sigil (stack canary) for this thread
        let sigil = Self::generate_sigil();

        // Dummy entry point (not used for user threads)
        fn dummy_entry() -> ! {
            loop {
                unsafe {
                    core::arch::asm!("hlt", options(nomem, nostack));
                }
            }
        }

        Self {
            id,
            state: ThreadState::Resting,
            priority,
            entry_point: dummy_entry,
            context,
            stack_bottom: 0,  // User stack, we don't track it in Thread
            stack_top: 0,
            sigil,
            vessel_id,
            resource_usage: ResourceUsage::default(),
            harmony_score: 1.0,
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

    /// Get the VesselId this thread belongs to (if any)
    pub fn vessel_id(&self) -> Option<VesselId> {
        self.vessel_id
    }

    /// Set the VesselId this thread belongs to
    pub fn set_vessel_id(&mut self, vessel_id: Option<VesselId>) {
        self.vessel_id = vessel_id;
    }
}

/// Tracks a thread's resource consumption
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceUsage {
    pub cpu_time: u64,
    pub memory_allocated: usize,
    pub messages_sent: u64,
}
