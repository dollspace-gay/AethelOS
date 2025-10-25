//! The Scheduler - The core of the Loom of Fate

use super::context::{switch_context_cooperative, context_switch_first, ThreadContext};
use super::harmony::{HarmonyAnalyzer, HarmonyMetrics};
use super::stack::Stack;
use super::thread::{Thread, ThreadId, ThreadPriority, ThreadState};
use super::LoomError;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const MAX_THREADS: usize = 1024;

/// The harmony-based cooperative/preemptive scheduler
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

    // === Preemptive Multitasking Support ===
    /// Is preemptive scheduling enabled?
    preemption_enabled: bool,
    /// Time quantum in timer ticks (e.g., 10 ticks = 10ms if timer is 1ms)
    time_quantum: u64,
    /// Ticks remaining in current thread's quantum
    quantum_remaining: u64,
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

        // Pre-allocate capacity to prevent reallocation during push
        // This avoids memory overlap between Vec storage and stack allocations
        let mut threads = Vec::with_capacity(16);
        unsafe { serial_out(b'b'); } // threads Vec created
        let mut stacks = Vec::with_capacity(16);
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
            // Preemption disabled by default (cooperative mode)
            preemption_enabled: false,
            time_quantum: 100,  // Default: 100ms quantum (conservative for testing)
            quantum_remaining: 100,
        }
    }

    /// Create a new Scheduler directly in a Box on the heap
    /// This avoids stack overflow by never creating the Scheduler on the stack
    pub fn new_boxed() -> alloc::boxed::Box<Self> {
        unsafe { serial_out(b'a'); }

        // Allocate uninitialized box
        let mut boxed: alloc::boxed::Box<core::mem::MaybeUninit<Self>> = alloc::boxed::Box::new_uninit();
        unsafe { serial_out(b'b'); }

        // Initialize fields directly in the box using ptr::addr_of_mut!
        // This ensures we never create intermediate references to uninitialized memory
        unsafe {
            let ptr: *mut Scheduler = boxed.as_mut_ptr();

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).threads), Vec::with_capacity(16));
            serial_out(b'c');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).stacks), Vec::with_capacity(16));
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

            // Initialize preemption fields (disabled by default, 100ms quantum for testing)
            core::ptr::write(core::ptr::addr_of_mut!((*ptr).preemption_enabled), false);
            serial_out(b'k');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).time_quantum), 100);
            serial_out(b'l');

            core::ptr::write(core::ptr::addr_of_mut!((*ptr).quantum_remaining), 100);
            serial_out(b'm');

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

        // CRITICAL: Keep stack alive IMMEDIATELY to prevent allocator from reusing this memory
        self.stacks.push(stack);

        // Create the thread with its stack
        let thread = Thread::new(thread_id, entry_point, priority, stack_bottom, stack_top);

        self.threads.push(thread);
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
    /// Prepare for yielding: select next thread and get context pointers
    /// Returns (should_switch, from_context_ptr, to_context_ptr)
    /// This method is designed to be called with the lock held, then the lock
    /// can be dropped before the actual context switch.
    pub fn prepare_yield(&mut self) -> (bool, *mut ThreadContext, *const ThreadContext) {
        unsafe { serial_out(b'P'); } // Entering prepare_yield (NEW CODE)
        unsafe { serial_out(b'1'); }

        // Analyze harmony before scheduling
        let metrics = self.harmony_analyzer.analyze(&mut self.threads);
        self.latest_metrics = metrics;
        unsafe { serial_out(b'2'); } // Harmony analyzed

        // Adaptive scheduling based on system harmony
        if metrics.system_harmony < 0.5 {
            // System is in disharmony - prioritize cooperative threads
            self.rebalance_for_harmony();
        }
        unsafe { serial_out(b'3'); } // After harmony check

        // Find the next thread to run
        let next_thread_id = self.select_next_thread();
        unsafe { serial_out(b'4'); } // After select

        if next_thread_id.is_none() {
            // No threads ready - this shouldn't happen in a well-designed system
            // but if it does, we just return (stay on current thread if any)
            unsafe { serial_out(b'0'); } // No next thread!
            return (false, core::ptr::null_mut(), core::ptr::null());
        }

        let next_id = next_thread_id.unwrap();
        unsafe { serial_out(b'5'); } // Got next_id

        // If we're switching to a different thread, prepare for context switch
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

            // Get raw pointers to the contexts
            let from_idx = self.threads.iter().position(|t| t.id() == current_id).unwrap();
            let to_idx = self.threads.iter().position(|t| t.id() == next_id).unwrap();

            let from_ctx_ptr = &mut self.threads[from_idx].context as *mut ThreadContext;
            let to_ctx_ptr = &self.threads[to_idx].context as *const ThreadContext;

            // Update current thread ID
            self.current_thread = Some(next_id);
            self.context_switches += 1;

            (true, from_ctx_ptr, to_ctx_ptr)
        } else {
            // Same thread or no current thread - don't switch
            (false, core::ptr::null_mut(), core::ptr::null())
        }
    }

    /// Called after a context switch to do any cleanup
    pub fn after_yield(&mut self) {
        // Currently nothing to do here, but this provides a hook for future cleanup
        unsafe { serial_out(b'6'); } // After yield complete
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

        // Perform the cooperative context switch
        // This will save the current state to from_ctx and restore to_ctx
        unsafe {
            switch_context_cooperative(from_ctx_ptr, to_ctx_ptr);
        }

        // When we return here, we're running as the "to" thread
        // (or possibly some other thread that later switched back to us)
    }

    /// Restore the first thread's context
    ///
    /// This is used when starting the very first thread. Unlike a normal context switch,
    /// we don't save the current context (since we're coming from the boot/init code).
    /// We just restore the thread's context and jump to it.
    ///
    /// # Safety
    /// This function never returns normally - it restores the thread's context
    fn restore_first_thread(&mut self, to_id: ThreadId) -> ! {
        // Find the thread's context
        let to_idx = self.threads.iter().position(|t| t.id() == to_id).unwrap();
        let entry_point = self.threads[to_idx].entry_point;
        let stack_top = self.threads[to_idx].stack_top;

        // Set up and jump to the thread
        // We enable interrupts so the thread can receive timer/keyboard interrupts
        unsafe {
            core::arch::asm!(
                "mov rsp, {stack}",       // Set up the new thread's stack
                "xor rbp, rbp",            // Clear frame pointer (indicates bottom of call stack)
                "sti",                     // Enable interrupts
                "jmp {entry}",             // Jump directly (no return address needed for fn() -> !)
                stack = in(reg) stack_top,
                entry = in(reg) entry_point,
                options(noreturn)
            );
        }
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

    /// Select the next thread to run - simple round-robin
    fn select_next_thread(&mut self) -> Option<ThreadId> {
        // Just pop the front of the queue - simple round-robin
        self.ready_queue.pop_front()
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

    /// DEBUG: Print all threads and their contexts
    pub fn debug_print_threads(&self) {
        crate::println!("DEBUG scheduler: Total threads = {}", self.threads.len());
        for (i, thread) in self.threads.iter().enumerate() {
            crate::println!("  Thread[{}]: id={}, rsp={:#x}, rip={:#x}, ctx_addr={:p}",
                i, thread.id().0, thread.context().rsp, thread.context().rip, thread.context());
        }
    }

    /// Get the context pointer for a specific thread
    ///
    /// Returns a raw pointer to the thread's context structure.
    /// Used for the Great Hand-Off to get the idle thread's context.
    pub fn get_thread_context(&self, thread_id: ThreadId) -> Option<*const ThreadContext> {
        let thread_idx = self.threads.iter()
            .position(|t| t.id() == thread_id)?;

        Some(&self.threads[thread_idx].context as *const ThreadContext)
    }

    /// Start weaving - The Sacred First Weave (DEPRECATED - use Great Hand-Off instead)
    ///
    /// This is the one-time ritual that transitions from the bootstrap code
    /// (which is not a real thread) to the first actual thread. Unlike normal
    /// context switches, this performs a one-way jump without saving state.
    ///
    /// # Safety
    /// This function never returns - it performs a one-way jump to the first thread
    pub fn start_weaving(&mut self) -> ! {
        // Select the first thread to run (highest priority thread from ready queue)
        let next_id = self.select_next_thread()
            .expect("Cannot start weaving - no threads in ready queue!");

        // Mark this thread as currently running
        self.current_thread = Some(next_id);

        // Update the thread's state to Weaving
        if let Some(thread) = self.find_thread_mut(next_id) {
            thread.set_state(ThreadState::Weaving);
            thread.record_time_slice();
            thread.last_run_time = crate::attunement::timer::ticks();
        }

        self.context_switches += 1;

        // Get the thread's context
        let thread_idx = self.threads.iter().position(|t| t.id() == next_id).unwrap();
        let context_ptr = &self.threads[thread_idx].context as *const ThreadContext;

        // Perform the sacred one-way context switch to the first thread
        // This never returns - we are now in the thread's world
        unsafe {
            context_switch_first(context_ptr);
        }
    }

    /// Prepare for the Great Hand-Off
    ///
    /// Sets the idle thread as the current thread and removes it from the ready queue.
    /// This must be called before context_switch_first to ensure proper scheduler state.
    pub fn prepare_handoff(&mut self, thread_id: ThreadId) {
        // Remove thread from ready queue since it's about to become current
        self.ready_queue.retain(|&id| id != thread_id);

        // Set it as the current thread
        self.current_thread = Some(thread_id);

        // Mark it as Weaving (running)
        if let Some(thread) = self.find_thread_mut(thread_id) {
            thread.set_state(ThreadState::Weaving);
        }
    }

    // === Preemptive Multitasking Control ===

    /// Enable preemptive multitasking with the given time quantum
    ///
    /// # Arguments
    /// * `quantum_ms` - Time quantum in milliseconds (e.g., 10 = 10ms per thread)
    ///
    /// When enabled, the timer interrupt will trigger context switches
    /// after the quantum expires, even if the thread hasn't yielded.
    pub fn enable_preemption(&mut self, quantum_ms: u64) {
        self.preemption_enabled = true;
        self.time_quantum = quantum_ms;
        self.quantum_remaining = quantum_ms;
    }

    /// Disable preemptive multitasking (return to cooperative mode)
    ///
    /// Threads will only switch when they explicitly call yield_now().
    pub fn disable_preemption(&mut self) {
        self.preemption_enabled = false;
    }

    /// Check if the current thread's quantum has expired and should be preempted
    ///
    /// Returns true if:
    /// - Preemption is enabled
    /// - Current thread's quantum has expired (quantum_remaining == 0)
    pub fn should_preempt(&mut self) -> bool {
        if !self.preemption_enabled {
            return false;
        }

        if self.quantum_remaining == 0 {
            // Quantum expired! Reset for next thread
            self.quantum_remaining = self.time_quantum;
            return true;
        }

        false
    }

    /// Decrement the current thread's quantum (called on each timer tick)
    ///
    /// This is called from the timer interrupt handler to track how much
    /// time the current thread has used.
    pub fn tick_quantum(&mut self) {
        if self.preemption_enabled && self.quantum_remaining > 0 {
            self.quantum_remaining -= 1;
        }
    }

    /// Check if preemption is currently enabled
    pub fn is_preemption_enabled(&self) -> bool {
        self.preemption_enabled
    }

    /// Get the current time quantum setting
    pub fn get_time_quantum(&self) -> u64 {
        self.time_quantum
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
