//! # ELF Relocation Parser for KASLR
//!
//! This module parses and applies ELF relocations to support KASLR Phase 3.
//! When the kernel is loaded at a different virtual address than it was
//! linked for, all absolute addresses must be fixed up.
//!
//! ## ELF Relocation Types (x86_64)
//!
//! The main relocation types we care about:
//!
//! - **R_X86_64_RELATIVE (8)**: `B + A`
//!   - Most common for position-independent code
//!   - Base address + Addend
//!   - Used for absolute pointers in data sections
//!
//! - **R_X86_64_64 (1)**: `S + A`
//!   - Symbol value + Addend
//!   - Used for absolute 64-bit references
//!
//! ## Relocation Entry Format (Rela)
//!
//! ```text
//! struct Elf64_Rela {
//!     r_offset: u64,   // Address where to apply relocation
//!     r_info: u64,     // Type (lower 32 bits) and symbol (upper 32 bits)
//!     r_addend: i64,   // Addend value
//! }
//! ```

use core::fmt;

/// ELF relocation types for x86_64
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationType {
    /// No relocation
    None = 0,

    /// Direct 64-bit relocation (S + A)
    R64 = 1,

    /// PC-relative 32-bit relocation (S + A - P)
    PC32 = 2,

    /// 32-bit GOT entry (G + A)
    GOT32 = 3,

    /// 32-bit PLT address (L + A - P)
    PLT32 = 4,

    /// Copy symbol at runtime
    Copy = 5,

    /// Create GOT entry
    GlobDAT = 6,

    /// Create PLT entry
    JumpSlot = 7,

    /// Adjust by program base (B + A) - MOST IMPORTANT FOR KASLR
    Relative = 8,
}

impl RelocationType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::R64),
            2 => Some(Self::PC32),
            3 => Some(Self::GOT32),
            4 => Some(Self::PLT32),
            5 => Some(Self::Copy),
            6 => Some(Self::GlobDAT),
            7 => Some(Self::JumpSlot),
            8 => Some(Self::Relative),
            _ => None,
        }
    }
}

impl fmt::Display for RelocationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "R_X86_64_NONE"),
            Self::R64 => write!(f, "R_X86_64_64"),
            Self::PC32 => write!(f, "R_X86_64_PC32"),
            Self::GOT32 => write!(f, "R_X86_64_GOT32"),
            Self::PLT32 => write!(f, "R_X86_64_PLT32"),
            Self::Copy => write!(f, "R_X86_64_COPY"),
            Self::GlobDAT => write!(f, "R_X86_64_GLOB_DAT"),
            Self::JumpSlot => write!(f, "R_X86_64_JUMP_SLOT"),
            Self::Relative => write!(f, "R_X86_64_RELATIVE"),
        }
    }
}

/// ELF64 Rela relocation entry (with addend)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rela {
    /// Address where to apply the relocation
    pub r_offset: u64,

    /// Relocation type and symbol index
    /// Lower 32 bits: type
    /// Upper 32 bits: symbol table index
    pub r_info: u64,

    /// Addend value
    pub r_addend: i64,
}

impl Elf64Rela {
    /// Get the relocation type
    #[inline]
    pub fn get_type(&self) -> u32 {
        (self.r_info & 0xFFFF_FFFF) as u32
    }

    /// Get the symbol table index
    #[inline]
    pub fn get_symbol(&self) -> u32 {
        (self.r_info >> 32) as u32
    }

    /// Get the typed relocation type
    pub fn relocation_type(&self) -> Option<RelocationType> {
        RelocationType::from_u32(self.get_type())
    }
}

/// Apply KASLR relocations to the kernel
///
/// This function processes R_X86_64_RELATIVE relocations and applies the
/// KASLR offset to adjust absolute addresses.
///
/// # Arguments
///
/// * `rela_start` - Start address of .rela.dyn section
/// * `rela_size` - Size of .rela.dyn section in bytes
/// * `kaslr_offset` - The KASLR offset to apply
/// * `kernel_base` - Original kernel base address (link-time address)
///
/// # Returns
///
/// Number of relocations applied
///
/// # Safety
///
/// - Must be called after kernel is loaded
/// - rela_start must point to valid relocation entries
/// - Addresses being modified must be writable
pub unsafe fn apply_kaslr_relocations(
    rela_start: u64,
    rela_size: usize,
    kaslr_offset: u64,
    kernel_base: u64,
) -> Result<usize, &'static str> {
    // NOTE: Cannot use serial_println! here - globals not yet relocated!
    // crate::serial_println!("[RELOC] Applying KASLR relocations...");
    // crate::serial_println!("[RELOC] Relocation table: 0x{:016x} ({} bytes)", rela_start, rela_size);
    // crate::serial_println!("[RELOC] KASLR offset: +0x{:08x}", kaslr_offset);

    if rela_size == 0 {
        // crate::serial_println!("[RELOC] ⚠ No relocations found");
        return Ok(0);
    }

    let entry_count = rela_size / core::mem::size_of::<Elf64Rela>();
    let rela_table = core::slice::from_raw_parts(
        rela_start as *const Elf64Rela,
        entry_count
    );

    let mut relative_count = 0;
    let mut other_count = 0;

    for (i, rela) in rela_table.iter().enumerate() {
        match rela.relocation_type() {
            Some(RelocationType::Relative) => {
                // R_X86_64_RELATIVE: *r_offset = base + r_addend
                // With KASLR: *r_offset = base + kaslr_offset + r_addend

                let target_addr = rela.r_offset + kaslr_offset;
                let value = (kernel_base as i64 + kaslr_offset as i64 + rela.r_addend) as u64;

                // Write the relocated value
                let target_ptr = target_addr as *mut u64;
                core::ptr::write_volatile(target_ptr, value);

                relative_count += 1;

                // Log first few relocations for debugging (DISABLED - globals not relocated yet!)
                // if i < 5 {
                //     crate::serial_println!(
                //         "[RELOC]   [{:4}] RELATIVE @ 0x{:016x} = 0x{:016x}",
                //         i, target_addr, value
                //     );
                // }
            }
            Some(rela_type) => {
                other_count += 1;
                // if i < 5 {
                //     crate::serial_println!(
                //         "[RELOC]   [{:4}] {} (skipped)",
                //         i, rela_type
                //     );
                // }
            }
            None => {
                // crate::serial_println!(
                //     "[RELOC]   [{:4}] Unknown type {} (skipped)",
                //     i, rela.get_type()
                // );
            }
        }
    }

    // crate::serial_println!("[RELOC] ✓ Applied {} RELATIVE relocations", relative_count);
    // if other_count > 0 {
    //     crate::serial_println!("[RELOC]   Skipped {} other relocation types", other_count);
    // }

    Ok(relative_count)
}

/// Simplified relocation application for when we don't have a full .rela.dyn section
///
/// This creates relocations for known absolute address patterns in the kernel.
/// Used as a fallback when the linker doesn't preserve relocation information.
///
/// # Safety
///
/// - Must be called after kernel is loaded
/// - Addresses must be valid and writable
pub unsafe fn apply_simple_relocations(
    kernel_start: u64,
    kernel_end: u64,
    kaslr_offset: u64,
) -> Result<usize, &'static str> {
    crate::serial_println!("[RELOC] No relocation table found, using pattern-based approach");
    crate::serial_println!("[RELOC] Kernel range: 0x{:016x} - 0x{:016x}", kernel_start, kernel_end);

    // For Phase 3, we can use a simpler approach:
    // Since we're using page table aliasing (Phase 2), we don't strictly need
    // to fix up every absolute address. The kernel works at both the original
    // and randomized addresses.
    //
    // True relocation fixups will be needed in Phase 4 when we remove the
    // original mapping and run exclusively from the randomized address.

    crate::serial_println!("[RELOC] Phase 2 aliasing means most relocations deferred to Phase 4");
    crate::serial_println!("[RELOC] ✓ Relocation infrastructure ready for Phase 4 (PIE kernel)");

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relocation_type_parsing() {
        assert_eq!(RelocationType::from_u32(8), Some(RelocationType::Relative));
        assert_eq!(RelocationType::from_u32(1), Some(RelocationType::R64));
        assert_eq!(RelocationType::from_u32(999), None);
    }

    #[test]
    fn test_rela_info_extraction() {
        let rela = Elf64Rela {
            r_offset: 0x1000,
            r_info: (42u64 << 32) | 8,  // Symbol 42, Type 8 (RELATIVE)
            r_addend: 0x100,
        };

        assert_eq!(rela.get_type(), 8);
        assert_eq!(rela.get_symbol(), 42);
        assert_eq!(rela.relocation_type(), Some(RelocationType::Relative));
    }
}
