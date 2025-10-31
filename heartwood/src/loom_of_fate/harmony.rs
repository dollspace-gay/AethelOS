//! Harmony analysis - Detecting and soothing parasitic behavior

use super::thread::{Thread, ThreadState};

/// Number of historical metrics to keep (fixed-size ring buffer)
const HARMONY_HISTORY_SIZE: usize = 100;

/// Analyzes system harmony and detects parasitic threads
pub struct HarmonyAnalyzer {
    /// Historical metrics for trend analysis (Fixed-size ring buffer - NO ALLOCATION)
    /// This is critical: we CANNOT allocate in interrupt context!
    history: [HarmonyMetrics; HARMONY_HISTORY_SIZE],
    history_index: usize,
    history_count: usize,
}

impl Default for HarmonyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HarmonyAnalyzer {
    pub fn new() -> Self {
        Self {
            history: [HarmonyMetrics::default(); HARMONY_HISTORY_SIZE],
            history_index: 0,
            history_count: 0,
        }
    }

    /// Analyze the harmony of all threads and update their scores
    pub fn analyze(&mut self, threads: &mut [Thread]) -> HarmonyMetrics {
        let total_threads = threads.len() as f32;
        if total_threads == 0.0 {
            return HarmonyMetrics::default();
        }

        // Calculate system-wide metrics
        let active_threads = threads
            .iter()
            .filter(|t| t.state() == ThreadState::Weaving)
            .count() as f32;

        let avg_harmony: f32 = threads.iter().map(|t| t.harmony_score()).sum::<f32>() / total_threads;

        let parasites = threads.iter().filter(|t| t.is_parasite()).count();

        // Update individual thread harmony scores
        for thread in threads.iter_mut() {
            let new_score = self.calculate_thread_harmony(thread);
            thread.set_harmony_score(new_score);
        }

        let metrics = HarmonyMetrics {
            average_harmony: avg_harmony,
            active_thread_ratio: active_threads / total_threads,
            parasite_count: parasites,
            system_harmony: self.calculate_system_harmony(avg_harmony, active_threads / total_threads),
        };

        // --- CRITICAL FIX: NO ALLOCATION IN INTERRUPT CONTEXT ---
        // Use fixed-size ring buffer instead of Vec::push() which can allocate
        // This prevents deadlock when timer interrupt fires during disk I/O allocation
        self.history[self.history_index] = metrics;
        self.history_index = (self.history_index + 1) % HARMONY_HISTORY_SIZE;
        if self.history_count < HARMONY_HISTORY_SIZE {
            self.history_count += 1;
        }
        // --- END CRITICAL FIX ---

        metrics
    }

    /// Calculate harmony score for a single thread
    fn calculate_thread_harmony(&self, thread: &Thread) -> f32 {
        let mut harmony: f32 = 1.0;

        // Penalize excessive resource usage
        let usage = thread.resource_usage();
        if usage.cpu_time > 1000 {
            harmony *= 0.9;
        }
        if usage.memory_allocated > 10 * 1024 * 1024 {
            harmony *= 0.9;
        }

        // Reward yielding behavior
        if thread.yields > 0 {
            harmony *= 1.1;
        }

        harmony.clamp(0.0, 1.0)
    }

    /// Calculate overall system harmony
    fn calculate_system_harmony(&self, avg_thread_harmony: f32, active_ratio: f32) -> f32 {
        // System harmony is a weighted combination of:
        // - Average thread harmony (70%)
        // - Balanced thread activity (30%)
        let balance_score = 1.0 - (active_ratio - 0.5).abs() * 2.0;
        (avg_thread_harmony * 0.7 + balance_score * 0.3).clamp(0.0, 1.0)
    }

    /// Determine if a thread should be throttled (soothed)
    pub fn should_soothe(&self, thread: &Thread) -> bool {
        thread.is_parasite()
    }

    /// Calculate throttle factor for a parasitic thread (0.0 - 1.0)
    pub fn soothe_factor(&self, thread: &Thread) -> f32 {
        if !thread.is_parasite() {
            return 1.0;
        }

        // More parasitic = more throttling
        thread.harmony_score()
    }
}

/// Metrics about system harmony
#[derive(Debug, Clone, Copy)]
pub struct HarmonyMetrics {
    /// Average harmony score across all threads (0.0 - 1.0)
    pub average_harmony: f32,

    /// Ratio of active threads to total threads
    pub active_thread_ratio: f32,

    /// Number of threads exhibiting parasitic behavior
    pub parasite_count: usize,

    /// Overall system harmony score (0.0 - 1.0)
    pub system_harmony: f32,
}

impl Default for HarmonyMetrics {
    fn default() -> Self {
        Self {
            average_harmony: 1.0,
            active_thread_ratio: 0.0,
            parasite_count: 0,
            system_harmony: 1.0,
        }
    }
}
