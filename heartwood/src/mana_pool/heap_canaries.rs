//! Heap Canaries - The Weaver's Sigil for Heap Protection
//!
//! Protects heap allocations from buffer overflows by placing canary values
//! before and after each allocation. If a buffer overflow occurs, it will
//! corrupt a canary, which is detected when the memory is freed.
//!
//! ## Philosophy
//! Just as threads are protected by the Weaver's Sigil on the stack,
//! heap allocations are guarded by sigils woven into the fabric of memory.
//! Each allocation is wrapped in protective marks that reveal corruption.

use crate::mana_pool::entropy::ChaCha8Rng;
use core::sync::atomic::{AtomicU64, Ordering};

/// Size of each canary (8 bytes)
pub const CANARY_SIZE: usize = 8;

/// Total overhead per allocation (pre-canary + post-canary)
pub const TOTAL_CANARY_OVERHEAD: usize = CANARY_SIZE * 2;

/// Global canary secret (XORed with allocation address for uniqueness)
static CANARY_SECRET: AtomicU64 = AtomicU64::new(0);

/// Statistics for heap canary violations
static VIOLATIONS_DETECTED: AtomicU64 = AtomicU64::new(0);

/// Enable/disable heap canary checking
static CANARIES_ENABLED: AtomicU64 = AtomicU64::new(0);

/// Check if heap canaries are currently enabled
#[inline(always)]
pub fn are_enabled() -> bool {
    CANARIES_ENABLED.load(Ordering::Acquire) != 0
}

/// Initialize heap canary system
///
/// # Safety
/// Must be called exactly once during kernel initialization
pub unsafe fn init() {
    // Generate a random canary secret
    let mut rng = ChaCha8Rng::from_hardware_fast();
    let secret = ((rng.next_u32() as u64) << 32) | (rng.next_u32() as u64);
    CANARY_SECRET.store(secret, Ordering::Relaxed);

    // Enable heap canaries
    CANARIES_ENABLED.store(1, Ordering::Release);
}

/// Generate a canary value for a specific allocation address
///
/// The canary is unique per-allocation by XORing the secret with the address.
/// This prevents an attacker from learning the canary from one allocation
/// and using it to bypass protection in another.
#[inline(always)]
pub fn generate_canary(addr: usize) -> u64 {
    let secret = CANARY_SECRET.load(Ordering::Relaxed);
    secret ^ (addr as u64)
}

/// Write canaries around an allocation
///
/// Memory layout:
/// ```
/// [PRE_CANARY][USER DATA][POST_CANARY]
///  8 bytes     size bytes  8 bytes
/// ```
///
/// # Safety
/// - `addr` must point to valid memory with at least `size + TOTAL_CANARY_OVERHEAD` bytes
/// - Memory must be writable
pub unsafe fn write_canaries(addr: usize, size: usize) {
    let canary = generate_canary(addr);

    // Write pre-canary before user data
    let pre_canary_ptr = addr as *mut u64;
    core::ptr::write_volatile(pre_canary_ptr, canary);

    // Write post-canary after user data
    let post_canary_ptr = (addr + CANARY_SIZE + size) as *mut u64;
    core::ptr::write_volatile(post_canary_ptr, canary);
}

/// Verify canaries and detect heap corruption
///
/// Returns true if canaries are intact, false if corrupted
///
/// # Safety
/// - `addr` must point to the start of an allocation (including pre-canary)
/// - Memory must be readable
pub unsafe fn verify_canaries(addr: usize, size: usize) -> bool {
    // Skip verification if canaries not enabled yet (during early boot)
    if CANARIES_ENABLED.load(Ordering::Acquire) == 0 {
        return true;
    }

    let expected_canary = generate_canary(addr);

    // Check pre-canary
    let pre_canary_ptr = addr as *const u64;
    let pre_canary = core::ptr::read_volatile(pre_canary_ptr);

    // Check post-canary
    let post_canary_ptr = (addr + CANARY_SIZE + size) as *const u64;
    let post_canary = core::ptr::read_volatile(post_canary_ptr);

    if pre_canary != expected_canary {
        log_violation(addr, size, "PRE-CANARY", expected_canary, pre_canary);
        return false;
    }

    if post_canary != expected_canary {
        log_violation(addr, size, "POST-CANARY", expected_canary, post_canary);
        return false;
    }

    true
}

/// Log a canary violation
unsafe fn log_violation(_addr: usize, _size: usize, _location: &str, _expected: u64, _found: u64) {
    // Track violation count
    // Detailed logging is done in the panic message when deallocation fails
    VIOLATIONS_DETECTED.fetch_add(1, Ordering::Relaxed);
}

/// Get the number of heap canary violations detected since boot
pub fn violations_count() -> u64 {
    VIOLATIONS_DETECTED.load(Ordering::Relaxed)
}

/// Get the current canary secret (for debugging only)
///
/// # Security Warning
/// This should NEVER be exposed to userspace!
pub fn get_canary_secret() -> u64 {
    CANARY_SECRET.load(Ordering::Relaxed)
}
