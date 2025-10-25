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

    // DEBUG: Silent marker that we've awakened
    unsafe {
        let mut port = 0x3f8u16;
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") b'!' as u8,  // ! = Made it to idle_thread!
            options(nomem, nostack, preserves_flags)
        );
    }

    // CRITICAL: Set ourselves as the current thread in the scheduler FIRST
    // The Great Hand-Off doesn't do this, so we must do it ourselves
    // DO THIS BEFORE ENABLING INTERRUPTS to avoid deadlock!
    unsafe {
        use super::{get_loom, ThreadId};
        let mut loom = get_loom().lock();
        let idle_id = ThreadId(1);
        loom.prepare_handoff(idle_id);

        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'S' as u8,  // S = Set as current
            options(nomem, nostack, preserves_flags)
        );
    }

    // NOW enable interrupts - the world can now interact with us
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    // DEBUG: Interrupts enabled
    unsafe {
        let mut port = 0x3f8u16;
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") b'I' as u8,  // I = Interrupts enabled
            options(nomem, nostack, preserves_flags)
        );
    }

    // The eternal, silent loop of the idle thread
    // We speak not. We hold no locks. We are the void between actions.
    loop {
        // DEBUG: Mark loop iteration
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") b'.' as u8,
                options(nomem, nostack, preserves_flags)
            );
        }

        // Yield to let other threads run
        yield_now();

        // DEBUG: After yield
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") b'y' as u8,
                options(nomem, nostack, preserves_flags)
            );
        }

        // Halt CPU until next interrupt (most power-efficient)
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }

        // DEBUG: After hlt
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3f8u16,
                in("al") b'h' as u8,
                options(nomem, nostack, preserves_flags)
            );
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
    // DEBUG: Mark that we started
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'K' as u8,  // K = Keyboard thread started
            options(nomem, nostack, preserves_flags)
        );
    }

    // TEMPORARILY DISABLED: println hangs in write_fmt
    // crate::println!("  ⟡ Keyboard thread awakened");

    // DEBUG: Can we reach the loop?
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'L' as u8,  // L = Reached loop
            options(nomem, nostack, preserves_flags)
        );
    }

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
    // DEBUG: Mark that we started
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'T' as u8,  // T = sHell Thread started
            options(nomem, nostack, preserves_flags)
        );
    }

    // FORCE interrupts enabled at thread start
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    // Debug: Check if interrupts are enabled NOW
    unsafe {
        let flags: u64;
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) flags,
            options(nomem, preserves_flags)
        );
        let interrupts_enabled = (flags & 0x200) != 0;

        let mut port = 0x3f8u16;
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") if interrupts_enabled { b'+' } else { b'-' },  // + = enabled, - = disabled
            options(nomem, nostack, preserves_flags)
        );
    }

    // TEMPORARILY DISABLED: println hangs in write_fmt
    // crate::println!("  ⟡ Shell thread awakened");
    // crate::println!();
    // display_welcome();

    // DEBUG: Shell thread continuing
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16,
            in("al") b'S' as u8,  // S = Shell continuing
            options(nomem, nostack, preserves_flags)
        );
    }

    // Display the initial shell prompt
    // TEMPORARILY DISABLED: print! also hangs in write_fmt
    // crate::eldarin::display_prompt();

    // Debug: mark that shell loop started
    unsafe {
        let mut port = 0x3f8u16;
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") b'L' as u8,  // L = Loop started
            options(nomem, nostack, preserves_flags)
        );
    }

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

    // NOTE: Removed stats display to avoid spinlock deadlock with interrupts
    // The stats() function locks the scheduler, and if an interrupt fires
    // while that lock is held, we get a deadlock.
    // TODO: Implement interrupt-safe stats() or cache stats before threads start

    crate::println!();
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
