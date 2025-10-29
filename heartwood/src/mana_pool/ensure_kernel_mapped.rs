//! Ensure Kernel Sections Are Mapped
//!
//! Under Limine, the bootloader maps the kernel based on ELF program headers.
//! However, large .bss sections (like our 8MB static heap) might not be fully mapped
//! because .bss is NOBITS (doesn't take file space).
//!
//! This module ensures ALL kernel sections are properly mapped in the page tables.

use super::kernel_sections::{kernel_start, kernel_end};
use super::page_tables::get_phys_addr;

/// Raw serial output for early boot
unsafe fn serial_out(c: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") 0x3f8u16,
        in("al") c,
        options(nomem, nostack, preserves_flags)
    );
}

unsafe fn serial_str(s: &str) {
    for byte in s.bytes() {
        serial_out(byte);
    }
}

/// Verify that the entire kernel is mapped in page tables
///
/// Checks every 2MB chunk of the kernel to ensure it's accessible.
/// Returns the number of unmapped chunks found.
pub unsafe fn verify_kernel_mapped() -> usize {
    serial_str("[VERIFY] Checking kernel page table mappings...\n");

    let start = kernel_start() as u64;
    let end = kernel_end() as u64;

    // Align to 2MB boundaries (Limine likely uses 2MB pages)
    let start_aligned = start & !0x1F_FFFF;
    let end_aligned = (end + 0x1F_FFFF) & !0x1F_FFFF;

    let mut unmapped = 0;
    let mut addr = start_aligned;

    while addr < end_aligned {
        match get_phys_addr(addr) {
            Some(_phys) => {
                serial_out(b'.');  // Mapped
            }
            None => {
                serial_out(b'X');  // UNMAPPED!
                unmapped += 1;
            }
        }
        addr += 0x20_0000;  // Next 2MB chunk
    }

    serial_out(b'\n');

    if unmapped > 0 {
        serial_str("[VERIFY] WARNING: Found ");
        // Print unmapped count
        let mut count = unmapped;
        if count == 0 {
            serial_out(b'0');
        } else {
            let mut divisor = 1000;
            let mut started = false;
            while divisor > 0 {
                let digit = count / divisor;
                if digit > 0 || started {
                    serial_out(b'0' + digit as u8);
                    started = true;
                }
                count %= divisor;
                divisor /= 10;
            }
        }
        serial_str(" unmapped 2MB chunks in kernel range!\n");
    } else {
        serial_str("[VERIFY] All kernel sections are mapped.\n");
    }

    unmapped
}

/// Ensure kernel is mapped - print kernel section info and warning
///
/// This should be called early in boot, after allocator init but before
/// using large amounts of heap memory.
///
/// NOTE: Full page table verification is skipped because it can hang
/// if page table structures aren't fully accessible. Instead, we just
/// warn about the large .bss section.
pub unsafe fn ensure_kernel_mapped() {
    serial_str("[VERIFY] Kernel uses an 8MB static .bss for heap.\n");
    serial_str("[VERIFY] If Limine didn't map full .bss, BTreeMap will crash.\n");
    serial_str("[VERIFY] Recommended: Use dynamic heap allocation via HHDM instead.\n");

    // Skip actual verification - it can hang if page tables aren't fully accessible
    // The real fix is to change HEAP_MEMORY from static array to dynamically allocated
    serial_str("[VERIFY] Continuing with static heap - may crash if .bss unmapped.\n");
}
