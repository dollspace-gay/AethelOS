//! # The Rune of Permanence
//!
//! Hardware-enforced immutability for critical kernel data structures.
//!
//! After the kernel completes initialization (the "Dawn of Awakening"), critical
//! data structures are marked as read-only at the hardware level using MMU page
//! table protection. This prevents any runtime modification—even by the kernel itself.
//!
//! ## Philosophy
//!
//! *"The fundamental laws of the realm, once scribed at the Dawn of Awakening, are
//! immutable. These crystalline structures, etched into the fabric of reality, cannot
//! be altered—for to change them would be to rewrite the very physics of the world."*
//!
//! ## Security Model
//!
//! The Rune of Permanence defends against data-only attacks:
//! - Function pointer table overwrites
//! - Security policy corruption
//! - Critical constant modification
//!
//! Protection is enforced by the MMU at the hardware level:
//! - Writes to sealed structures cause immediate page faults
//! - Zero performance overhead (hardware-enforced)
//! - Cannot be bypassed in software

use core::sync::atomic::{AtomicBool, Ordering};

/// Whether the Rune of Permanence has been sealed
static RUNE_SEALED: AtomicBool = AtomicBool::new(false);

/// Check if the Rune of Permanence has been sealed
#[inline(always)]
pub fn is_sealed() -> bool {
    RUNE_SEALED.load(Ordering::Acquire)
}

/// Get the boundaries of the .rune section
///
/// Returns (start_address, end_address) of the .rune section as defined
/// in the linker script.
pub fn get_rune_boundaries() -> (u64, u64) {
    extern "C" {
        static __rune_start: u8;
        static __rune_end: u8;
    }

    unsafe {
        let start = &__rune_start as *const u8 as u64;
        let end = &__rune_end as *const u8 as u64;
        (start, end)
    }
}

/// Get the size of the .rune section in bytes
pub fn get_rune_size() -> u64 {
    let (start, end) = get_rune_boundaries();
    end - start
}

/// Get the number of pages in the .rune section
pub fn get_rune_page_count() -> u64 {
    let size = get_rune_size();
    // Round up to nearest page (4KB = 0x1000)
    (size + 0xFFF) / 0x1000
}

/// Seal the .rune section as read-only after boot
///
/// This marks all pages in the .rune section as read-only in the page tables,
/// preventing any further modification to structures placed in that section.
///
/// # Safety
/// This MUST only be called once, after all permanent structures have been
/// initialized. After this call, ANY writes to the .rune section will cause
/// a page fault.
///
/// # Panics
/// - If called more than once
/// - If the .rune section is empty
/// - If the .rune section is not page-aligned
pub unsafe fn seal_rune_section() {
    // Prevent double-sealing
    if RUNE_SEALED.swap(true, Ordering::AcqRel) {
        panic!("◈ FATAL: Attempted to seal Rune of Permanence twice!");
    }

    let (start, end) = get_rune_boundaries();

    // Validate section exists and is non-empty
    if start >= end {
        panic!(
            "◈ FATAL: Invalid .rune section (start=0x{:x}, end=0x{:x})",
            start, end
        );
    }

    // Validate page alignment
    if start % 0x1000 != 0 {
        panic!(
            "◈ FATAL: .rune section start (0x{:x}) is not page-aligned!",
            start
        );
    }

    // Log sealing operation via println
    crate::println!("◈ Sealing The Rune of Permanence...");
    crate::println!("  Range: 0x{:016x} - 0x{:016x}", start, end);
    crate::println!("  Size: {} bytes ({} KB)", end - start, (end - start) / 1024);
    crate::println!("  Pages: {}", get_rune_page_count());

    // Mark rune as sealed in security policy (last write before MMU protection)
    super::security_policy::mark_rune_sealed();

    // Make the .rune section read-only at the MMU level
    match super::page_tables::make_readonly(start, end) {
        Ok(pages_modified) => {
            crate::println!("  ✓ Modified {} page(s) to read-only", pages_modified);
        }
        Err(e) => {
            panic!("◈ FATAL: Failed to seal .rune section: {}", e);
        }
    }

    // Flush TLB to ensure changes take effect immediately
    super::page_tables::flush_tlb();

    crate::println!("  ✓ The Rune is sealed. Permanence enforced by the MMU.");
}

/// Test helper: Place a test variable in the .rune section
#[link_section = ".rune"]
static mut TEST_RUNE_VAR: u64 = 0xDEADBEEF;

/// Verify the .rune section is properly configured
///
/// This function performs basic sanity checks on the .rune section to ensure
/// it was properly created by the linker.
///
/// # Tests
/// - Section exists and has non-zero size
/// - Section is page-aligned
/// - Test variable is actually in the section
pub fn verify_rune_section() -> bool {
    let (start, end) = get_rune_boundaries();

    // Check section exists
    if start >= end {
        return false;
    }

    // Check page alignment
    if start % 0x1000 != 0 {
        return false;
    }

    // Check test variable is in range
    let test_addr = &raw const TEST_RUNE_VAR as u64;
    if test_addr < start || test_addr >= end {
        return false;
    }

    // Check we can read the test variable
    let value = unsafe { TEST_RUNE_VAR };
    if value != 0xDEADBEEF {
        return false;
    }

    true
}

/// Display information about the .rune section
pub fn display_rune_info() {
    let (start, end) = get_rune_boundaries();
    let size = get_rune_size();
    let pages = get_rune_page_count();
    let sealed = is_sealed();

    crate::println!();
    crate::println!("◈ The Rune of Permanence");
    crate::println!();
    crate::println!("  Status: {}", if sealed { "✓ SEALED" } else { "○ UNSEALED (boot in progress)" });
    crate::println!("  Protection: MMU Read-Only Pages");
    crate::println!();
    crate::println!("  .rune section:");
    crate::println!("    Address: 0x{:016x} - 0x{:016x}", start, end);
    crate::println!("    Size: {} bytes ({} KB)", size, size / 1024);
    crate::println!("    Pages: {} (4KB pages)", pages);
    crate::println!();

    if sealed {
        crate::println!("  The foundational laws remain unbroken. The Rune stands eternal.");
    } else {
        crate::println!("  The Rune awaits sealing after the Dawn of Awakening.");
    }
}
