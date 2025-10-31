//! # System Threads
//!
//! The foundational threads that give AethelOS life.
//! These threads are the first to awaken, the servants of the system,
//! each with a sacred purpose.
//!
//! ## Philosophy
//! System threads exist to serve, not to dominate.
//! The idle thread waits patiently, consuming nothing.
//! The keyboard thread listens attentively to user intentions.
//! The shell thread translates human wishes into system actions.

use super::yield_now;

/// The Idle Thread - The First Awakening
///
/// This is the FIRST true thread to execute. The bootstrap ghost hand-offs
/// control to this thread, which then performs the system awakening.
///
/// After awakening the system, it becomes the patient servant that runs
/// only when no other thread can run.
///
/// Priority: Idle (lowest)
/// Harmony: Perfect (1.0) - it exists only to serve
pub fn idle_thread() -> ! {
    // --- THE AWAKENING ---
    // We are now the first true, managed thread. The bootstrap ghost is gone.
    //
    // THE VOW OF SILENCE:
    // The idle thread is pure. It holds no locks, prints nothing, demands nothing.
    // It exists only to yield the CPU when all other threads are sleeping.

    // CRITICAL: Set ourselves as the current thread in the scheduler FIRST
    // The Great Hand-Off doesn't do this, so we must do it ourselves
    // DO THIS BEFORE ENABLING INTERRUPTS to avoid deadlock!
    unsafe {
        // DEBUG: Mark function entry
        for &byte in b"[FUNC:idle_prep_handoff]".iter() {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }

        use super::{get_loom, ThreadId};
        let mut loom = get_loom().lock();
        let idle_id = ThreadId(1);
        loom.prepare_handoff(idle_id);
    }

    // NOW enable interrupts - the world can now interact with us
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    // The eternal, silent loop of the idle thread
    // We speak not. We hold no locks. We are the void between actions.
    loop {
        // Yield to let other threads run
        yield_now();

        // Halt CPU until next interrupt (most power-efficient)
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// The Keyboard Thread - The Listener
///
/// This thread processes keyboard input, translating scancodes
/// into meaningful events. It listens patiently for keystrokes
/// and forwards them to interested parties.
///
/// Priority: High (user input is important)
/// Harmony: High - it serves user needs directly
pub fn keyboard_thread() -> ! {
    crate::println!("  ⟡ Keyboard thread awakened");

    loop {
        // In a message-passing system, this would:
        // 1. Wait for keyboard interrupt message from Nexus
        // 2. Process the scancode into a KeyEvent
        // 3. Send KeyEvent to shell or focused application

        // For now, we just yield - the keyboard interrupt handler
        // already echoes characters directly
        yield_now();

        // Small delay to avoid spinning too fast
        // In a real implementation, this thread would block waiting for messages
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// The Shell Thread - The Interpreter
///
/// This thread provides the interactive shell (Eldarin),
/// where users commune with AethelOS. It displays prompts,
/// processes commands, and orchestrates the system's response.
///
/// Priority: Normal (important but not critical)
/// Harmony: High - it translates user intentions
pub fn shell_thread() -> ! {
    // FORCE interrupts enabled at thread start
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::println!("  ⟡ Shell thread awakened");
    crate::println!();
    display_welcome();

    // Display the initial shell prompt
    crate::eldarin::display_prompt();

    // Shell loop - poll for commands that were buffered by interrupt handler
    loop {
        // Check if a command is ready (newline was pressed)
        crate::eldarin::poll();

        // Yield to other threads
        yield_now();

        // Small delay to avoid spinning
        for _ in 0..500000 {  // Longer delay so we can see poll() calls
            core::hint::spin_loop();
        }
    }
}

/// Display the welcome message
fn display_welcome() {
    crate::println!("====================================================================");
    crate::println!("                                                                    ");
    crate::println!("                     Welcome to AethelOS                            ");
    crate::println!("                                                                    ");
    crate::println!("          The Operating System of Symbiotic Computing               ");
    crate::println!("                                                                    ");
    crate::println!("   \"We do not command the machine; we dance with it.\"              ");
    crate::println!("                                                                    ");
    crate::println!("====================================================================");
    crate::println!();

    // Display system statistics
    // NOTE: This is now interrupt-safe! The stats() function uses without_interrupts()
    // to disable interrupts while holding the scheduler lock, preventing deadlocks.
    display_system_stats();

    crate::println!();
}

/// Display system statistics (threads and memory)
///
/// This is interrupt-safe because:
/// - loom_of_fate::stats() uses without_interrupts()
/// - Memory allocator stats use interrupt-safe locks
fn display_system_stats() {
    // Get scheduler stats
    let loom_stats = crate::loom_of_fate::stats();

    // Get memory stats
    let mana_stats = crate::mana_pool::stats();

    crate::println!("   System Status:");
    crate::println!("   ─────────────");

    // Thread statistics
    crate::println!("   • Threads: {} total ({} weaving, {} resting)",
        loom_stats.total_threads,
        loom_stats.weaving_threads,
        loom_stats.resting_threads);

    crate::println!("   • Harmony: {:.2} (system: {:.2})",
        loom_stats.average_harmony,
        loom_stats.system_harmony);

    if loom_stats.parasite_count > 0 {
        crate::println!("   • Parasites detected: {}", loom_stats.parasite_count);
    }

    crate::println!("   • Context switches: {}", loom_stats.context_switches);

    // Memory statistics
    let used_kb = mana_stats.sanctuary_used / 1024;
    let total_kb = mana_stats.sanctuary_total / 1024;
    let used_percent = if total_kb > 0 {
        (used_kb * 100) / total_kb
    } else {
        0
    };

    crate::println!("   • Memory: {} KB / {} KB ({}% used)",
        used_kb, total_kb, used_percent);

    crate::println!("   • Objects: {}", mana_stats.total_objects);
}


/// A demonstration thread that counts upward
///
/// This is useful for testing that multiple threads are actually
/// switching and making progress.
pub fn demo_counter_thread() -> ! {
    crate::println!("  ⟡ Demo counter thread awakened");

    let mut count = 0u64;

    loop {
        if count % 1000 == 0 {
            crate::println!("  [Counter: {}]", count);
        }

        count += 1;

        // Yield every 100 iterations
        if count % 100 == 0 {
            yield_now();
        }
    }
}

/// A demonstration thread that displays dots periodically
///
/// This provides visual feedback that the system is alive and threads
/// are making progress.
pub fn demo_heartbeat_thread() -> ! {
    crate::println!("  ⟡ Demo heartbeat thread awakened");

    loop {
        // Wait a bit
        for _ in 0..100000 {
            core::hint::spin_loop();
        }

        crate::print!(".");

        yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These threads run forever, so we can't really unit test them
    // They're tested by actually running the system

    #[test]
    fn test_thread_signatures() {
        // Just verify the function signatures are correct
        let _idle: fn() -> ! = idle_thread;
        let _keyboard: fn() -> ! = keyboard_thread;
        let _shell: fn() -> ! = shell_thread;
    }
}
