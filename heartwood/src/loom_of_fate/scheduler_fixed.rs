//! The Scheduler - The core of the Loom of Fate

use super::context::{switch_context, ThreadContext};
use super::harmony::{HarmonyAnalyzer, HarmonyMetrics};
use super::stack::Stack;
use super::thread::{Thread, ThreadId, ThreadPriority, ThreadState};
use super::LoomError;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const MAX_THREADS: usize = 1024;

/// The harmony-based cooperative scheduler
pub struct Scheduler {
    threads: Vec<Thread>,
    stacks: Vec<Stack>,  // Stack storage (owned by scheduler)
    ready_queue: VecDeque<ThreadId>,
    current_thread: Option<ThreadId>,
    next_thread_id: u64,
    harmony_analyzer: HarmonyAnalyzer,
    /// Latest harmony metrics from the analyzer
    latest_metrics: HarmonyMetrics,
    /// Total number of context switches performed
    context_switches: u64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to write to serial for debugging
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

impl Scheduler {
    pub fn new() -> Self {
        unsafe { serial_out(b'a'); } // Scheduler::new() started
        let threads = Vec::new();
        unsafe { serial_out(b'b'); } // threads Vec created
        let stacks = Vec::new();
        unsafe { serial_out(b'c'); } // stacks Vec created
        let ready_queue = VecDeque::new();
        unsafe { serial_out(b'd'); } // ready_queue created
        let harmony_analyzer = HarmonyAnalyzer::new();
        unsafe { serial_out(b'e'); } // harmony_analyzer created

        unsafe { serial_out(b'f'); } // About to return
        Self {
            threads,
            stacks,
            ready_queue,
            current_thread: None,
            next_thread_id: 1,
            harmony_analyzer,
            latest_metrics: HarmonyMetrics::default(),
            context_switches: 0,
        }
    }

    /// Create a new Scheduler directly in a Box on the heap
    /// This avoids stack overflow by never creating the Scheduler on the stack
    pub fn new_boxed() -> alloc::boxed::Box<Self> {
        unsafe { serial_out(b'a'); }

        // Allocate uninitialized box
        let mut boxed = alloc::boxed::Box::new_uninit();
        unsafe { serial_out(b'b'); }

        // Initialize fields directly in the box using ptr::addr_of_mut!
        // This ensures we never create intermediate references to uninitialized memory
        unsafe {
            let ptr = boxed.as_mut_ptr();

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).threads), Vec::new());
            serial_out(b'c');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).stacks), Vec::new());
            serial_out(b'd');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).ready_queue), VecDeque::new());
            serial_out(b'e');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).current_thread), None);
            serial_out(b'f');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).next_thread_id), 1);
            serial_out(b'g');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).harmony_analyzer), HarmonyAnalyzer::new());
            serial_out(b'h');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).latest_metrics), HarmonyMetrics::default());
            serial_out(b'i');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).context_switches), 0);
            serial_out(b'j');

            boxed.assume_init()
        }
    }

    /// Spawn a new thread
    pub fn spawn(&mut self, entry_point: fn() -> !, priority: ThreadPriority) -> Result<ThreadId, LoomError> {
        if self.threads.len() >= MAX_THREADS {
            return Err(LoomError::OutOfThreads);
        }

        let thread_id = ThreadId(self.next_thread_id);
        self.next_thread_id += 1;

        // Allocate a stack for this thread
        let stack = Stack::new().ok_or(LoomError::StackAllocationFailed)?;
        let stack_bottom = stack.bottom();
        let stack_top = stack.top();

        // Create the thread with its stack
        let thread = Thread::new(thread_id, entry_point, priority, stack_bottom, stack_top);

        self.threads.push(thread);
        self.stacks.push(stack);  // Keep stack alive
        self.ready_queue.push_back(thread_id);

        Ok(thread_id)
    }

    /// Yield the current thread and switch to the next one
    ///
    /// This is the heart of cooperative multitasking. The current thread
    /// voluntarily gives up the CPU, and we select the next thread based
    /// on harmony and priority.
    ///
    /// # Safety
    /// This function performs context switching which involves raw register
    /// manipulation. It should only be called from safe contexts where
    /// the scheduler state is valid.
    pub fn yield_current(&mut self) {
        // Analyze harmony before scheduling
        let metrics = self.harmony_analyzer.analyze(&mut self.threads);
        self.latest_metrics = metrics;

        // Adaptive scheduling based on system harmony
        if metrics.system_harmony < 0.5 {
            // System is in disharmony - prioritize cooperative threads
            self.rebalance_for_harmony();
        }

        // Find the next thread to run
        let next_thread_id = self.select_next_thread();

        if next_thread_id.is_none() {
            // No threads ready - this shouldn't happen in a well-designed system
            // but if it does, we just return (stay on current thread if any)
            return;
        }

        let next_id = next_thread_id.unwrap();

        // If we're switching to a different thread, perform context switch
        if self.current_thread.is_some() && self.current_thread != Some(next_id) {
            let current_id = self.current_thread.unwrap();

            // Update current thread state
            if let Some(current_thread) = self.find_thread_mut(current_id) {
                current_thread.record_yield();
                current_thread.set_state(ThreadState::Resting);
            }

            // Add current thread back to ready queue
            self.ready_queue.push_back(current_id);

            // Update next thread state
            if let Some(next_thread) = self.find_thread_mut(next_id) {
                next_thread.set_state(ThreadState::Weaving);
                next_thread.record_time_slice();
                next_thread.last_run_time = crate::attunement::timer::ticks();
            }

            // Perform the actual context switch
            self.context_switches += 1;
            self.perform_context_switch(current_id, next_id);

            // After we return from context switch, we're running as the "next" thread
            // (which may actually be this thread again after future switches)
        } else if self.current_thread.is_none() {
            // First thread to run - just start it (no context to save)
            if let Some(next_thread) = self.find_thread_mut(next_id) {
                next_thread.set_state(ThreadState::Weaving);
                next_thread.record_time_slice();
                next_thread.last_run_time = crate::attunement::timer::ticks();
            }

            self.current_thread = Some(next_id);
            // Jump to the first thread (this will not return)
            self.jump_to_thread(next_id);
        }

        // Update current thread ID
        self.current_thread = Some(next_id);
    }

    /// Perform a context switch between two threads
    ///
    /// # Safety
    /// Assumes both thread IDs are valid and the threads exist
    fn perform_context_switch(&mut self, from_id: ThreadId, to_id: ThreadId) {
        // Get raw pointers to the contexts before borrowing
        let from_idx = self.threads.iter().position(|t| t.id() == from_id).unwrap();
        let to_idx = self.threads.iter().position(|t| t.id() == to_id).unwrap();

        let from_ctx_ptr = &mut self.threads[from_idx].context as *mut ThreadContext;
        let to_ctx_ptr = &self.threads[to_idx].context as *const ThreadContext;

        // Perform the context switch
        // This will save the current state to from_ctx and restore to_ctx
        unsafe {
            switch_context(from_ctx_ptr, to_ctx_ptr);
        }

        // When we return here, we're running as the "to" thread
        // (or possibly some other thread that later switched back to us)
    }

    /// Jump to a thread for the first time (no context to save)
    ///
    /// # Safety
    /// This function never returns - it jumps to the thread's entry point
    fn jump_to_thread(&mut self, to_id: ThreadId) -> ! {
        // Find the thread's context
        let to_idx = self.threads.iter().position(|t| t.id() == to_id).unwrap();
        let to_ctx_ptr = &self.threads[to_idx].context as *const ThreadContext;

        // Create a dummy context for the "from" side (we won't use it)
        let mut dummy_ctx = ThreadContext::empty();
        let dummy_ctx_ptr = &mut dummy_ctx as *mut ThreadContext;

        // Jump to the new thread
        // Note: switch_context will try to save our state to dummy_ctx,
        // but since we're never coming back, that's fine
        unsafe {
            switch_context(dummy_ctx_ptr, to_ctx_ptr);
        }

        // Never reached
        unreachable!("jump_to_thread should never return");
    }

    /// Rebalance the ready queue when system harmony is low
    /// This promotes cooperative threads and demotes parasitic ones
    fn rebalance_for_harmony(&mut self) {
        // Collect thread info from ready queue
        let mut queue_info: Vec<(ThreadId, f32)> = self
            .ready_queue
            .iter()
            .filter_map(|&id| {
                self.find_thread(id).map(|t| (id, t.harmony_score()))
            })
            .collect();

        // Sort by harmony score (highest first)
        queue_info.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        // Rebuild ready queue with harmony-prioritized order
        self.ready_queue.clear();
        for (id, _) in queue_info {
            self.ready_queue.push_back(id);
        }
    }

    /// Select the next thread to run based on harmony
    fn select_next_thread(&mut self) -> Option<ThreadId> {
        if self.ready_queue.is_empty() {
            return None;
        }

        // Sort ready queue by harmony score and priority
        let mut candidates: Vec<_> = self
            .ready_queue
            .iter()
            .filter_map(|&id| {
                self.find_thread(id).map(|t| {
                    (
                        id,
                        t.priority(),
                        t.harmony_score(),
                    )
                })
            })
            .collect();

        candidates.sort_by(|a, b| {
            // First by priority, then by harmony score
            a.1.cmp(&b.1).then(b.2.partial_cmp(&a.2).unwrap())
        });

        candidates.first().map(|(id, _, _)| {
            // Remove from ready queue
            self.ready_queue.retain(|&tid| tid != *id);
            *id
        })
    }

    /// Find a thread by ID
    fn find_thread(&self, id: ThreadId) -> Option<&Thread> {
        self.threads.iter().find(|t| t.id() == id)
    }

    /// Find a thread by ID (mutable)
    fn find_thread_mut(&mut self, id: ThreadId) -> Option<&mut Thread> {
        self.threads.iter_mut().find(|t| t.id() == id)
    }

    /// Get the current thread ID
    pub fn current_thread_id(&self) -> Option<ThreadId> {
        self.current_thread
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        // Use the latest metrics from the analyzer
        SchedulerStats {
            total_threads: self.threads.len(),
            weaving_threads: self
                .threads
                .iter()
                .filter(|t| t.state() == ThreadState::Weaving)
                .count(),
            resting_threads: self
                .threads
                .iter()
                .filter(|t| t.state() == ThreadState::Resting)
                .count(),
            tangled_threads: self
                .threads
                .iter()
                .filter(|t| t.state() == ThreadState::Tangled)
                .count(),
            average_harmony: self.latest_metrics.average_harmony,
            system_harmony: self.latest_metrics.system_harmony,
            parasite_count: self.latest_metrics.parasite_count,
            context_switches: self.context_switches,
        }
    }

    /// Get the latest harmony metrics
    pub fn harmony_metrics(&self) -> HarmonyMetrics {
        self.latest_metrics
    }
}

/// Statistics about the scheduler
#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    pub total_threads: usize,
    pub weaving_threads: usize,
    pub resting_threads: usize,
    pub tangled_threads: usize,
    pub average_harmony: f32,
    pub system_harmony: f32,
    pub parasite_count: usize,
    pub context_switches: u64,
}
