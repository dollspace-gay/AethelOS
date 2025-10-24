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

use super::{ThreadPriority, yield_now};

/// The Idle Thread - The Patient Servant
///
/// This thread runs only when no other thread can run.
/// It yields immediately, giving others every opportunity to work.
/// When truly alone, it halts the CPU to save energy.
///
/// Priority: Idle (lowest)
/// Harmony: Perfect (1.0) - it exists only to serve
pub fn idle_thread() -> ! {
    crate::println!("  ⟡ Idle thread awakened");

    loop {
        // Yield to any other thread that wants to run
        yield_now();

        // If we're still running after yielding, truly nothing else can run
        // Halt CPU until next interrupt (saves power)
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
    crate::println!("  ⟡ Shell thread awakened");
    crate::println!();
    display_welcome();
    display_prompt();

    // Shell loop
    loop {
        // In a full implementation, this would:
        // 1. Wait for keyboard events from keyboard thread
        // 2. Build up command buffer as user types
        // 3. Parse and execute commands when Enter is pressed
        // 4. Display results and new prompt

        // For now, just yield cooperatively
        yield_now();

        // Small delay
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// Display the welcome message
fn display_welcome() {
    crate::println!("╔══════════════════════════════════════════════════════════════════╗");
    crate::println!("║                                                                  ║");
    crate::println!("║                    Welcome to AethelOS                           ║");
    crate::println!("║                                                                  ║");
    crate::println!("║         The Operating System of Symbiotic Computing              ║");
    crate::println!("║                                                                  ║");
    crate::println!("║  \"We do not command the machine; we dance with it.\"              ║");
    crate::println!("║                                                                  ║");
    crate::println!("╚══════════════════════════════════════════════════════════════════╝");
    crate::println!();
    crate::println!("System Status:");

    // Display system information
    let stats = super::stats();
    crate::println!("  • Threads: {} active", stats.total_threads);
    crate::println!("  • System Harmony: {:.2}", stats.system_harmony);
    crate::println!("  • Context Switches: {}", stats.context_switches);
    crate::println!();
}

/// Display the shell prompt
fn display_prompt() {
    crate::print!("aethel> ");
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
