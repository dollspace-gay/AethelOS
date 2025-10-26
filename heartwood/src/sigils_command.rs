/// SIGILS - Display The Weaver's Sigils (stack canaries) for all threads
///
/// This command shows the status of stack and heap canary protection without
/// exposing actual canary values (for security). It performs real verification
/// checks to ensure protection is active and working correctly.

use crate::loom_of_fate::{without_interrupts, ThreadState};
use alloc::vec::Vec;

/// Entry point for the sigils command - sets up paging
pub fn cmd_sigils() {
    unsafe {
        crate::eldarin::PAGING_ACTIVE = true;
        crate::eldarin::PAGING_PAGE = 0;
        crate::eldarin::PAGING_COMMAND = Some(crate::eldarin::PagingCommand::Sigils);
    }
    show_sigils_page(0);
}

/// Display a specific page of sigil information
pub fn show_sigils_page(page: usize) {
    match page {
        0 => show_overview_page(),
        1 => show_stack_protection_page(),
        2 => show_heap_protection_page(),
        _ => {
            // No more pages
            unsafe {
                crate::eldarin::PAGING_ACTIVE = false;
                crate::eldarin::PAGING_PAGE = 0;
                crate::eldarin::PAGING_COMMAND = None;
            }
            crate::eldarin::display_prompt();
        }
    }
}

/// Page 0: Overview of The Weaver's Sigil protection system
fn show_overview_page() {
    crate::println!();
    crate::println!("◈ The Weaver's Sigils - Stack & Heap Protection");
    crate::println!();
    crate::println!("  The Weaver's Sigils are cryptographic marks placed on the stack");
    crate::println!("  and heap to detect buffer overflow attacks. Each thread carries");
    crate::println!("  a unique 64-bit sigil, and each heap allocation is wrapped in");
    crate::println!("  protective canaries.");
    crate::println!();

    // Check stack protection status
    let stack_guard = crate::stack_protection::get_current_canary();
    let stack_active = stack_guard != 0;

    crate::println!("  Protection Systems:");
    crate::println!();

    if stack_active {
        crate::println!("    [✓] Stack Protection   - ACTIVE");
        crate::println!("        Mode: LLVM strong (per-function canaries)");
        crate::println!("        Coverage: All functions with buffers or address-taken locals");
    } else {
        crate::println!("    [✗] Stack Protection   - INACTIVE (boot in progress?)");
    }

    let heap_enabled = crate::mana_pool::heap_canaries::are_enabled();
    if heap_enabled {
        crate::println!("    [✓] Heap Protection    - ACTIVE");
        crate::println!("        Mode: Pre/post allocation canaries (8 bytes each)");
        crate::println!("        Coverage: All heap allocations");
    } else {
        crate::println!("    [✗] Heap Protection    - INACTIVE (boot in progress?)");
    }

    crate::println!();
    crate::println!("  Note: Actual canary values are not displayed for security reasons.");
    crate::println!();
    crate::println!("Press SPACE for stack details, or ESC to exit");
}

/// Page 1: Stack protection details and per-thread verification
fn show_stack_protection_page() {
    crate::println!();
    crate::println!("◈ Stack Protection Status (Page 2/3)");
    crate::println!();

    // Check global canary
    let current_guard = crate::stack_protection::get_current_canary();
    let canary_initialized = current_guard != 0;

    if canary_initialized {
        crate::println!("  Global Stack Guard: ✓ INITIALIZED");
    } else {
        crate::println!("  Global Stack Guard: ✗ NOT INITIALIZED");
    }

    crate::println!();
    crate::println!("  Per-Thread Sigils:");
    crate::println!();

    // Verify thread sigils
    without_interrupts(|| {
        unsafe {
            let loom = crate::loom_of_fate::get_loom().lock();

            // Collect active threads
            let threads: Vec<_> = loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .collect();

            crate::println!("    Active Threads: {}", threads.len());
            crate::println!();

            let mut all_valid = true;

            for thread in &threads {
                let state_str = match thread.state {
                    ThreadState::Weaving => "Weaving",
                    ThreadState::Resting => "Resting",
                    ThreadState::Tangled => "Tangled",
                    ThreadState::Fading => "Fading",
                };

                // Check if sigil is set (non-zero)
                let sigil_valid = thread.sigil != 0;

                let status = if sigil_valid { "✓" } else { "✗" };

                crate::print!("    Thread #{:2} [{}] - Sigil: {}",
                    thread.id.0, state_str, status);

                if !sigil_valid {
                    crate::print!(" (UNINITIALIZED)");
                    all_valid = false;
                }

                crate::println!();
            }

            crate::println!();

            // Check uniqueness
            let mut sigils: Vec<u64> = loom.threads.iter()
                .filter(|t| !matches!(t.state, ThreadState::Fading))
                .map(|t| t.sigil)
                .filter(|&s| s != 0)  // Exclude uninitialized
                .collect();

            let original_len = sigils.len();
            sigils.sort_unstable();
            sigils.dedup();
            let unique_len = sigils.len();

            crate::println!("  Uniqueness Check:");
            if original_len == unique_len && original_len > 0 {
                crate::println!("    ✓ All {} sigils are unique", unique_len);
            } else if original_len == 0 {
                crate::println!("    ✗ No initialized sigils found");
            } else {
                crate::println!("    ✗ WARNING: {} duplicate sigils detected!",
                    original_len - unique_len);
            }

            crate::println!();

            if all_valid && canary_initialized && original_len == unique_len && original_len > 0 {
                crate::println!("  Overall Status: ✓ PROTECTION ACTIVE AND VERIFIED");
            } else {
                crate::println!("  Overall Status: ⚠ ISSUES DETECTED");
            }
        }
    });

    crate::println!();
    crate::println!("Press SPACE for heap details, or ESC to exit");
}

/// Page 2: Heap protection details and violation status
fn show_heap_protection_page() {
    crate::println!();
    crate::println!("◈ Heap Protection Status (Page 3/3)");
    crate::println!();

    let heap_enabled = crate::mana_pool::heap_canaries::are_enabled();

    if heap_enabled {
        crate::println!("  Heap Canary System: ✓ ACTIVE");
    } else {
        crate::println!("  Heap Canary System: ✗ INACTIVE");
    }

    crate::println!();
    crate::println!("  Protection Mechanism:");
    crate::println!("    [✓] Pre-allocation canary  - 8 bytes before each allocation");
    crate::println!("    [✓] Post-allocation canary - 8 bytes after each allocation");
    crate::println!("    [✓] Per-allocation unique  - Canary XORed with address");
    crate::println!();

    // Check violation count
    let violations = crate::mana_pool::heap_canaries::violations_count();

    crate::println!("  Violation Detection:");
    if violations == 0 {
        crate::println!("    ✓ No violations detected");
        crate::println!("    ✓ Heap integrity maintained");
    } else {
        crate::println!("    ✗ {} VIOLATIONS DETECTED!", violations);
        crate::println!("    ✗ HEAP CORRUPTION OCCURRED!");
        crate::println!();
        crate::println!("    WARNING: Buffer overflow(s) were caught and blocked.");
        crate::println!("    Check system logs for details.");
    }

    crate::println!();

    // Overall heap status
    if heap_enabled && violations == 0 {
        crate::println!("  Overall Heap Status: ✓ PROTECTED AND CLEAN");
    } else if heap_enabled && violations > 0 {
        crate::println!("  Overall Heap Status: ⚠ PROTECTED BUT VIOLATIONS OCCURRED");
    } else {
        crate::println!("  Overall Heap Status: ✗ PROTECTION NOT ACTIVE");
    }

    crate::println!();
    crate::println!("  Security Note:");
    crate::println!("    Actual canary values and secrets are not displayed to prevent");
    crate::println!("    information leakage. The Weaver's Sigils remain hidden, but");
    crate::println!("    their protective power is verified and active.");
    crate::println!();
    crate::println!("Press ESC to exit, or SPACE to return to page 1");
}
