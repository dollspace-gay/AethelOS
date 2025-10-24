//! The Scheduler - The core of the Loom of Fate

use super::harmony::{HarmonyAnalyzer, HarmonyMetrics};
use super::thread::{Thread, ThreadId, ThreadPriority, ThreadState};
use super::LoomError;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const MAX_THREADS: usize = 1024;

/// The harmony-based cooperative scheduler
pub struct Scheduler {
    threads: Vec<Thread>,
    ready_queue: VecDeque<ThreadId>,
    current_thread: Option<ThreadId>,
    next_thread_id: u64,
    harmony_analyzer: HarmonyAnalyzer,
    /// Latest harmony metrics from the analyzer
    latest_metrics: HarmonyMetrics,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
            ready_queue: VecDeque::new(),
            current_thread: None,
            next_thread_id: 1,
            harmony_analyzer: HarmonyAnalyzer::new(),
            latest_metrics: HarmonyMetrics::default(),
        }
    }

    /// Spawn a new thread
    pub fn spawn(&mut self, entry_point: fn() -> !, priority: ThreadPriority) -> Result<ThreadId, LoomError> {
        if self.threads.len() >= MAX_THREADS {
            return Err(LoomError::OutOfThreads);
        }

        let thread_id = ThreadId(self.next_thread_id);
        self.next_thread_id += 1;

        let thread = Thread::new(thread_id, entry_point, priority);
        self.threads.push(thread);
        self.ready_queue.push_back(thread_id);

        Ok(thread_id)
    }

    /// Yield the current thread
    pub fn yield_current(&mut self) {
        if let Some(current_id) = self.current_thread {
            if let Some(thread) = self.find_thread_mut(current_id) {
                thread.record_yield();
                thread.set_state(ThreadState::Resting);
            }

            self.ready_queue.push_back(current_id);
            self.current_thread = None;
        }

        self.schedule_next();
    }

    /// Schedule the next thread to run
    fn schedule_next(&mut self) {
        // Analyze harmony before scheduling
        let metrics = self.harmony_analyzer.analyze(&mut self.threads);
        self.latest_metrics = metrics;

        // Adaptive scheduling based on system harmony
        if metrics.system_harmony < 0.5 {
            // System is in disharmony - prioritize cooperative threads
            // and deprioritize parasites more aggressively
            self.rebalance_for_harmony();
        }

        // Find the next thread to run based on harmony
        let next_thread_id = self.select_next_thread();

        if let Some(thread_id) = next_thread_id {
            // Check if thread is parasitic and get soothe factor before mutable borrow
            let (is_parasitic, soothe_factor) = self.find_thread(thread_id)
                .map(|t| {
                    let parasitic = self.harmony_analyzer.should_soothe(t);
                    let factor = if parasitic {
                        self.harmony_analyzer.soothe_factor(t)
                    } else {
                        1.0
                    };
                    (parasitic, factor)
                })
                .unwrap_or((false, 1.0));

            if let Some(thread) = self.find_thread_mut(thread_id) {
                thread.set_state(ThreadState::Weaving);
                thread.record_time_slice();

                // If this thread is parasitic, soothe it based on harmony score
                if is_parasitic {
                    // In a real implementation, we would:
                    // 1. Reduce time slice based on soothe_factor (lower = more throttling)
                    // 2. Insert deliberate pauses/delays
                    // 3. Lower its effective priority
                    // For now, this documents the intent for future implementation
                    let _ = soothe_factor; // Acknowledge we have the factor
                }
            }

            self.current_thread = Some(thread_id);
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
}
