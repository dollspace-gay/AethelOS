//! Stack Protection Runtime Support
//!
//! This module provides the runtime support required by LLVM's stack protector.
//! When `-fstack-protector` is enabled, LLVM generates code that:
//! 1. Reads from `__stack_chk_guard` at function entry
//! 2. Places the value on the stack (the canary)
//! 3. Checks the value before function return
//! 4. Calls `__stack_chk_fail()` if the canary was corrupted

use core::sync::atomic::{AtomicU64, Ordering};

/// Global stack canary value (thread-local via context switch)
///
/// LLVM-generated code reads this value at function entry and checks it
/// at function exit. We update this value on every context switch to use
/// the current thread's unique Weaver's Sigil.
///
/// # Security
/// This value changes per-thread, making it harder for attackers to
/// predict the canary. However, an attacker with arbitrary read can
/// still extract it, which is why we combine this with ASLR and other
/// defenses.
#[no_mangle]
pub static __stack_chk_guard: AtomicU64 = AtomicU64::new(0xDEADBEEF_CAFEBABE);

/// Stack canary failure handler
///
/// This function is called by LLVM-generated code when a stack canary
/// check fails, indicating that a buffer overflow has corrupted the stack.
///
/// # Behavior
/// This function NEVER returns. It logs diagnostic information and then
/// panics to prevent the corrupted thread from continuing execution.
///
/// # Safety
/// This function is called by compiler-generated code in a potentially
/// corrupted context. We must not trust any stack-allocated data and
/// should minimize operations that might use corrupted state.
#[no_mangle]
pub extern "C" fn __stack_chk_fail() -> ! {
    // Log the violation (use serial port directly to avoid stack operations)
    unsafe {
        log_canary_violation();
    }

    // Panic with a clear message
    panic!("◈ STACK CANARY VIOLATION: The Weaver's Sigil has been corrupted!\n\
           \n\
           A buffer overflow has been detected. The thread's stack canary\n\
           was overwritten, indicating memory corruption. Execution cannot\n\
           continue safely.\n\
           \n\
           This protection prevented the overflow from hijacking control flow.\n\
           The Weaver's Sigil stands vigilant.");
}

/// Log diagnostic information about the canary violation
///
/// This function is called from `__stack_chk_fail()` to provide debugging
/// information about which thread detected the corruption.
///
/// # Safety
/// Must be called with interrupts disabled to avoid corruption of diagnostic
/// output. Avoids using the stack as much as possible.
unsafe fn log_canary_violation() {
    // Print to VGA (safer than serial which might use stack)
    crate::println!("\n╔════════════════════════════════════════════════════════╗");
    crate::println!("║  ⚠  STACK CANARY VIOLATION DETECTED  ⚠              ║");
    crate::println!("╚════════════════════════════════════════════════════════╝");
    crate::println!();
    crate::println!("  The Weaver's Sigil has been corrupted!");
    crate::println!("  A buffer overflow has overwritten the stack canary.");
    crate::println!();

    // Try to get current thread info (might fail if stack is corrupted)
    // Note: We can't use catch_unwind in no_std, so if this fails, we'll panic anyway
    if let Some(thread_id) = crate::loom_of_fate::current_thread() {
        crate::println!("  Thread ID: {}", thread_id.0);
    } else {
        crate::println!("  Thread ID: <unable to determine>");
    }

    crate::println!();
    crate::println!("  Expected canary: 0x{:016x}", __stack_chk_guard.load(Ordering::Relaxed));
    crate::println!("  Actual canary:   <corrupted>");
    crate::println!();
    crate::println!("  This overflow was BLOCKED by The Weaver's Sigil.");
    crate::println!("  Execution halted before control flow hijacking.");
    crate::println!();

    // Increment violation counter (if we have one in the future)
    // CANARY_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
}

/// Initialize the stack canary for a thread
///
/// This function should be called during context switch to update
/// `__stack_chk_guard` with the current thread's unique sigil.
///
/// # Arguments
/// * `sigil` - The thread's unique Weaver's Sigil (64-bit random value)
///
/// # Safety
/// This function must be called during context switch, with interrupts
/// disabled, to avoid race conditions where LLVM-generated code reads
/// a canary from one thread while executing another thread's code.
#[inline(always)]
pub unsafe fn set_current_canary(sigil: u64) {
    __stack_chk_guard.store(sigil, Ordering::Relaxed);
}

/// Get the current canary value
///
/// This is primarily for debugging and testing. Production code should
/// not need to read this value directly (LLVM-generated code handles it).
///
/// # Returns
/// The current value of `__stack_chk_guard`
#[inline(always)]
pub fn get_current_canary() -> u64 {
    __stack_chk_guard.load(Ordering::Relaxed)
}

/// Test helper: Simulate a stack canary violation
///
/// This function is used in tests to verify that canary checking works.
/// DO NOT call this in production code!
#[cfg(test)]
pub fn simulate_violation() -> ! {
    __stack_chk_fail()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_get_set() {
        unsafe {
            set_current_canary(0x1234567890ABCDEF);
        }
        assert_eq!(get_current_canary(), 0x1234567890ABCDEF);

        unsafe {
            set_current_canary(0xFEDCBA0987654321);
        }
        assert_eq!(get_current_canary(), 0xFEDCBA0987654321);
    }

    #[test]
    fn test_canary_default() {
        // Default value should be non-zero (our placeholder)
        let default = get_current_canary();
        assert_ne!(default, 0, "Canary should have non-zero default");
    }

    #[test]
    #[should_panic(expected = "STACK CANARY VIOLATION")]
    fn test_stack_chk_fail_panics() {
        simulate_violation();
    }
}
