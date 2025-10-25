//! # Timer - The Rhythm of Time

use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

/// Get the current tick count
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Increment the tick count (called from timer interrupt)
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Called on each timer tick from the interrupt handler
/// This is where we can trigger the scheduler to switch tasks
pub fn on_tick() {
    // Increment the global tick counter
    tick();

    // For now, we don't trigger scheduling from timer interrupts
    // The threads yield cooperatively
    // In a preemptive system, we would call the scheduler here
}
