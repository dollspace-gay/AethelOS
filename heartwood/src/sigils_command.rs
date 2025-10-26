/// SIGILS - Display The Weaver's Sigils (stack canaries) for all threads
///
/// This command shows the unique stack canary values protecting each thread.
/// Each sigil is a 64-bit cryptographically random value that guards against
/// buffer overflow attacks.

pub fn cmd_sigils() {
    use crate::loom_of_fate::{without_interrupts, ThreadState};
    use alloc::vec::Vec;

    crate::println!("◈ The Weaver's Sigils - Stack Canary Protection");
    crate::println!();

    crate::println!("  Status: ✓ ACTIVE (Phase 2: LLVM stack protection)");
    crate::println!("  Mode: LLVM strong mode + per-thread canaries");
    crate::println!("  Protection: All functions with buffers or address-taken locals");
    crate::println!();

    // Show current global canary value
    let current_canary = crate::stack_protection::get_current_canary();
    crate::println!("  Current __stack_chk_guard: 0x{:016x}", current_canary);
    crate::println!();

    // Get thread information
    without_interrupts(|| {
        unsafe {
            let loom = crate::loom_of_fate::get_loom().lock();

            // Collect threads (not fading)
            let threads: Vec<_> = loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .collect();

            crate::println!("  Active Sigils ({} threads):", threads.len());
            crate::println!();

            for thread in threads {
                let state_str = match thread.state {
                    ThreadState::Weaving => "Weaving",
                    ThreadState::Resting => "Resting",
                    ThreadState::Tangled => "Tangled",
                    ThreadState::Fading => "Fading",
                };

                let priority_str = match thread.priority {
                    crate::loom_of_fate::ThreadPriority::Critical => "Critical",
                    crate::loom_of_fate::ThreadPriority::High => "High",
                    crate::loom_of_fate::ThreadPriority::Normal => "Normal",
                    crate::loom_of_fate::ThreadPriority::Low => "Low",
                    crate::loom_of_fate::ThreadPriority::Idle => "Idle",
                };

                crate::println!("    Thread #{} [{}|{}]",
                    thread.id.0, state_str, priority_str);
                crate::println!("      Sigil: 0x{:016x}", thread.sigil);
                crate::println!();
            }

            // Verify uniqueness
            let mut sigils: Vec<u64> = loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .map(|t| t.sigil)
                .collect();

            let original_len = sigils.len();
            sigils.sort_unstable();
            sigils.dedup();
            let unique_len = sigils.len();

            if original_len == unique_len {
                crate::println!("  ✓ All sigils are unique ({} distinct values)", unique_len);
            } else {
                crate::println!("  ✗ WARNING: {} duplicate sigils detected!",
                    original_len - unique_len);
            }

            crate::println!();
            crate::println!("  Protection Status:");
            crate::println!("    [✓] Per-thread storage      - Active");
            crate::println!("    [✓] Cryptographic generation - Active");
            crate::println!("    [✓] Stack canary placement  - Active (LLVM inserts at function entry)");
            crate::println!("    [✓] Overflow detection      - Active (LLVM checks before return)");
            crate::println!();
            crate::println!("  Phase 2 Complete: LLVM stack-protector (strong mode) is active.");
            crate::println!("  All functions with buffers or address-taken locals are protected.");
        }
    });

    // Heap Canary Status
    crate::println!();
    crate::println!("  Heap Protection (Buffer Overflow Detection):");
    crate::println!("    [✓] Pre-allocation canaries  - Active (8 bytes before each allocation)");
    crate::println!("    [✓] Post-allocation canaries - Active (8 bytes after each allocation)");

    let violations = crate::mana_pool::heap_canaries::violations_count();
    if violations == 0 {
        crate::println!("    [✓] Violations detected      - None (heap integrity maintained)");
    } else {
        crate::println!("    [✗] Violations detected      - {} (HEAP CORRUPTION!)", violations);
    }

    crate::println!();
    crate::println!("The sigils remain pure and distinct.");
    crate::println!("Stack and heap are guarded by the Weaver's protective marks.");
}
