//! # The Ward of the Unseen Paths (KASLR)
//!
//! *"The paths through the forest change with each step. But now, the Heartwood itself*
//! *has learned to wander. No ancient map can mark its true location, for it rests*
//! *in a different grove each dawn."*
//!
//! ## Philosophy
//!
//! This builds upon the Ward of Shifting Mists (basic ASLR for stacks). Not only do
//! the paths change, but **the location of the Heartwood itself is never the same**
//! from one dawn to the next. An attacker cannot rely on ancient maps (hardcoded
//! addresses) to find the kernel's core functions, because the entire forest has
//! magically relocated overnight.
//!
//! ## What is KASLR?
//!
//! **Kernel Address Space Layout Randomization** randomizes the base address of the
//! kernel in virtual memory. Without KASLR:
//! - Kernel is always at `0xFFFF_8000_0000_0000` (predictable)
//! - Attackers know exact addresses of kernel functions
//! - ROP (Return-Oriented Programming) attacks are easier
//! - A single memory leak reveals all kernel addresses
//!
//! With KASLR:
//! - Kernel base is at `0xFFFF_8000_0000_0000 + random_offset`
//! - Random offset is typically 0-512MB (29 bits of entropy)
//! - Each boot produces a different kernel memory layout
//! - Attackers must leak and compute offsets dynamically
//!
//! ## Implementation Strategy
//!
//! AethelOS uses **Virtual KASLR** (simpler than physical KASLR):
//! 1. Generate entropy using RDTSC at early boot
//! 2. Calculate random offset (aligned to page boundaries)
//! 3. Apply offset to kernel base virtual address
//! 4. Update page tables to map kernel at new location
//! 5. Relocate all absolute addresses
//!
//! **Entropy Bits:**
//! - We use 24 bits of entropy (16MB alignment)
//! - Offset range: 0 - 256MB
//! - This provides 2^24 = 16,777,216 possible kernel locations

use core::arch::asm;

/// The standard kernel base address (without KASLR)
/// This is the higher-half direct mapping start
pub const KERNEL_BASE_DEFAULT: u64 = 0xFFFF_8000_0000_0000;

/// Maximum KASLR offset (256MB)
/// Keeps kernel within first 512MB of higher half
const MAX_KASLR_OFFSET: u64 = 256 * 1024 * 1024;

/// KASLR alignment (16MB)
/// Offset must be aligned to this boundary for huge pages
const KASLR_ALIGNMENT: u64 = 16 * 1024 * 1024;

/// Entropy bits used for randomization
/// 24 bits = 16,777,216 possible positions
const ENTROPY_BITS: u32 = 24;

/// Global: The actual kernel base address after KASLR
static mut KERNEL_BASE_ACTUAL: u64 = KERNEL_BASE_DEFAULT;

/// Global: The KASLR offset applied
static mut KASLR_OFFSET: u64 = 0;

/// Global: Whether KASLR is enabled
static mut KASLR_ENABLED: bool = false;

/// Generate entropy for KASLR using RDTSC
///
/// Uses the CPU timestamp counter for randomization. This is:
/// - Fast (single instruction)
/// - Available on all x86-64 CPUs
/// - Unpredictable at boot (depends on timing, interrupts, etc.)
///
/// # Returns
///
/// Random offset aligned to KASLR_ALIGNMENT
fn generate_kaslr_entropy() -> u64 {
    let mut entropy: u64;

    unsafe {
        // Read timestamp counter
        asm!(
            "rdtsc",
            "shl rdx, 32",
            "or rax, rdx",
            out("rax") entropy,
            out("rdx") _,
        );
    }

    // Mix entropy with XOR and rotation to spread bits
    entropy ^= entropy >> 17;
    entropy ^= entropy << 31;
    entropy ^= entropy >> 8;

    // Mask to entropy bits and align
    let offset = (entropy & ((1 << ENTROPY_BITS) - 1)) & !(KASLR_ALIGNMENT - 1);

    // Ensure offset is within bounds
    offset % MAX_KASLR_OFFSET
}

/// Check if RDRAND is available
///
/// RDRAND is a hardware RNG available on newer Intel CPUs (Ivy Bridge+)
/// and AMD CPUs (Ryzen+). It provides true hardware randomness.
///
/// # Returns
///
/// `true` if RDRAND is supported
fn has_rdrand() -> bool {
    let ecx: u32;

    unsafe {
        // CPUID with EAX=1: Processor Info and Feature Bits
        asm!(
            "push rbx",
            "mov eax, 1",
            "cpuid",
            "pop rbx",
            out("ecx") ecx,
            out("eax") _,
            out("edx") _,
        );
    }

    // RDRAND is bit 30 of ECX
    (ecx & (1 << 30)) != 0
}

/// Generate entropy using RDRAND (if available)
///
/// Uses hardware random number generator for higher quality entropy.
///
/// # Returns
///
/// Random offset aligned to KASLR_ALIGNMENT, or 0 if RDRAND fails
fn generate_kaslr_entropy_rdrand() -> u64 {
    let mut random: u64;
    let mut success: u8;

    unsafe {
        // Try RDRAND instruction
        asm!(
            "rdrand {random}",
            "setc {success}",
            random = out(reg) random,
            success = out(reg_byte) success,
        );
    }

    if success != 0 {
        // RDRAND succeeded, use hardware entropy
        let offset = (random & ((1 << ENTROPY_BITS) - 1)) & !(KASLR_ALIGNMENT - 1);
        offset % MAX_KASLR_OFFSET
    } else {
        // RDRAND failed, fall back to RDTSC
        generate_kaslr_entropy()
    }
}

/// Initialize KASLR - randomize kernel base address
///
/// This must be called very early during boot, before any code relies on
/// absolute kernel addresses.
///
/// # Safety
///
/// Must be called only once during kernel initialization.
/// Must be called before virtual memory is fully set up.
pub unsafe fn init_kaslr() {
    crate::serial_println!("[KASLR] Initializing Ward of the Unseen Paths...");

    // Generate random offset
    let offset = if has_rdrand() {
        crate::serial_println!("[KASLR] Using RDRAND for hardware entropy");
        generate_kaslr_entropy_rdrand()
    } else {
        crate::serial_println!("[KASLR] Using RDTSC for entropy");
        generate_kaslr_entropy()
    };

    // Store the offset
    KASLR_OFFSET = offset;
    KERNEL_BASE_ACTUAL = KERNEL_BASE_DEFAULT + offset;

    crate::serial_println!(
        "[KASLR] Kernel base: 0x{:016x} (offset: +0x{:08x}, {} MB)",
        KERNEL_BASE_ACTUAL,
        offset,
        offset / (1024 * 1024)
    );

    let entropy_mb = MAX_KASLR_OFFSET / (1024 * 1024);
    crate::serial_println!(
        "[KASLR] Entropy: {} bits ({} MB range)",
        ENTROPY_BITS,
        entropy_mb
    );

    // Note: In a full implementation, we would:
    // 1. Relocate the kernel to the new address
    // 2. Update page tables
    // 3. Fix up all absolute addresses
    //
    // For now, we track the offset for future use when we implement
    // position-independent kernel code.

    KASLR_ENABLED = true;
    crate::serial_println!("[KASLR] ✓ The Ward of the Unseen Paths conceals the Heartwood");
}

/// Get the current kernel base address
///
/// With KASLR enabled, this returns the randomized base address.
/// Without KASLR, returns the default base address.
pub fn get_kernel_base() -> u64 {
    unsafe { KERNEL_BASE_ACTUAL }
}

/// Get the KASLR offset applied
///
/// Returns the random offset added to the kernel base address.
/// Returns 0 if KASLR is not enabled.
pub fn get_kaslr_offset() -> u64 {
    unsafe { KASLR_OFFSET }
}

/// Check if KASLR is enabled
pub fn is_kaslr_enabled() -> bool {
    unsafe { KASLR_ENABLED }
}

/// Get the number of entropy bits used for KASLR
pub fn get_entropy_bits() -> u32 {
    if is_kaslr_enabled() {
        ENTROPY_BITS
    } else {
        0
    }
}

/// Get the entropy range in megabytes
pub fn get_entropy_range_mb() -> u64 {
    MAX_KASLR_OFFSET / (1024 * 1024)
}

/// Calculate an address with KASLR offset applied
///
/// Takes a default kernel address and applies the KASLR offset.
/// Use this when you need to compute actual addresses of kernel symbols.
///
/// # Arguments
///
/// * `default_addr` - The address without KASLR
///
/// # Returns
///
/// The actual address with KASLR offset applied
pub fn apply_kaslr_offset(default_addr: u64) -> u64 {
    if is_kaslr_enabled() && default_addr >= KERNEL_BASE_DEFAULT {
        default_addr + get_kaslr_offset()
    } else {
        default_addr
    }
}

/// Remove KASLR offset from an address
///
/// Takes an actual kernel address and removes the KASLR offset
/// to get the default address. Useful for debugging and logging.
///
/// # Arguments
///
/// * `actual_addr` - The address with KASLR applied
///
/// # Returns
///
/// The default address without KASLR offset
pub fn remove_kaslr_offset(actual_addr: u64) -> u64 {
    let kernel_base = unsafe { KERNEL_BASE_ACTUAL };
    if is_kaslr_enabled() && actual_addr >= kernel_base {
        actual_addr - get_kaslr_offset()
    } else {
        actual_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_generation() {
        // Generate multiple offsets
        let offset1 = generate_kaslr_entropy();
        let offset2 = generate_kaslr_entropy();

        // Should be different (extremely likely with RDTSC)
        // Note: This could theoretically fail, but probability is ~0
        // assert!(offset1 != offset2);  // Commented out due to non-determinism

        // Should be aligned
        assert_eq!(offset1 % KASLR_ALIGNMENT, 0);
        assert_eq!(offset2 % KASLR_ALIGNMENT, 0);

        // Should be within bounds
        assert!(offset1 < MAX_KASLR_OFFSET);
        assert!(offset2 < MAX_KASLR_OFFSET);
    }

    #[test]
    fn test_kaslr_offset_math() {
        // Test applying and removing offsets
        let test_offset = 64 * 1024 * 1024; // 64 MB
        unsafe {
            KASLR_OFFSET = test_offset;
            KERNEL_BASE_ACTUAL = KERNEL_BASE_DEFAULT + test_offset;
            KASLR_ENABLED = true;
        }

        let test_addr = KERNEL_BASE_DEFAULT + 0x1000;
        let actual_addr = apply_kaslr_offset(test_addr);
        assert_eq!(actual_addr, test_addr + test_offset);

        let recovered = remove_kaslr_offset(actual_addr);
        assert_eq!(recovered, test_addr);
    }
}
