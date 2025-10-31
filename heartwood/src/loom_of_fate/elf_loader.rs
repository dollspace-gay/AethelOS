//! # ELF Loader - The Gateway Between Worlds
//!
//! Loads ELF (Executable and Linkable Format) binaries into Vessels,
//! allowing them to manifest in the mortal realm (userspace).
//!
//! ## Philosophy
//!
//! An ELF file is a scroll of power, inscribed with the instructions
//! for creating a new consciousness. The loader reads this scroll,
//! interprets its sigils, and breathes life into a Vessel.
//!
//! ## Implementation
//!
//! Supports ELF64 (x86-64) format with:
//! - Program header loading (PT_LOAD segments)
//! - Position-independent executables (PIE)
//! - Basic validation and security checks

use alloc::vec::Vec;
use core::mem::size_of;

/// ELF magic number: \x7fELF
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class: 64-bit
const ELFCLASS64: u8 = 2;

/// ELF data encoding: little-endian
const ELFDATA2LSB: u8 = 1;

/// ELF version: current
const EV_CURRENT: u8 = 1;

/// ELF type: executable
const ET_EXEC: u16 = 2;

/// ELF type: position-independent executable
const ET_DYN: u16 = 3;

/// ELF machine: x86-64
const EM_X86_64: u16 = 62;

/// Program header type: loadable segment
const PT_LOAD: u32 = 1;

/// Program header flags: executable
const PF_X: u32 = 1;

/// Program header flags: writable
const PF_W: u32 = 2;

/// Program header flags: readable
const PF_R: u32 = 4;

/// ELF64 file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Ehdr {
    /// Magic number and class info
    pub e_ident: [u8; 16],
    /// Object file type (ET_EXEC, ET_DYN, etc.)
    pub e_type: u16,
    /// Target architecture (EM_X86_64)
    pub e_machine: u16,
    /// ELF version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size
    pub e_ehsize: u16,
    /// Program header entry size
    pub e_phentsize: u16,
    /// Program header entry count
    pub e_phnum: u16,
    /// Section header entry size
    pub e_shentsize: u16,
    /// Section header entry count
    pub e_shnum: u16,
    /// Section header string table index
    pub e_shstrndx: u16,
}

/// ELF64 program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    /// Segment type (PT_LOAD, PT_DYNAMIC, etc.)
    pub p_type: u32,
    /// Segment flags (PF_R, PF_W, PF_X)
    pub p_flags: u32,
    /// Segment file offset
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment physical address (unused on x86-64)
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

/// Errors that can occur during ELF loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Invalid ELF magic number
    InvalidMagic,
    /// Wrong ELF class (not 64-bit)
    InvalidClass,
    /// Wrong endianness (not little-endian)
    InvalidEndianness,
    /// Wrong ELF version
    InvalidVersion,
    /// Wrong ELF type (not executable or dynamic)
    InvalidType,
    /// Wrong machine architecture (not x86-64)
    InvalidMachine,
    /// File too small to contain headers
    FileTooSmall,
    /// Program header out of bounds
    ProgramHeaderOutOfBounds,
    /// Invalid alignment
    InvalidAlignment,
    /// Segment overlaps with kernel space
    SegmentOverlapsKernel,
    /// No loadable segments found
    NoLoadableSegments,
}

impl core::fmt::Display for ElfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ElfError::InvalidMagic => write!(f, "Invalid ELF magic number"),
            ElfError::InvalidClass => write!(f, "Not a 64-bit ELF file"),
            ElfError::InvalidEndianness => write!(f, "Not a little-endian ELF file"),
            ElfError::InvalidVersion => write!(f, "Invalid ELF version"),
            ElfError::InvalidType => write!(f, "Not an executable or PIE file"),
            ElfError::InvalidMachine => write!(f, "Not an x86-64 ELF file"),
            ElfError::FileTooSmall => write!(f, "ELF file too small"),
            ElfError::ProgramHeaderOutOfBounds => write!(f, "Program header out of bounds"),
            ElfError::InvalidAlignment => write!(f, "Invalid segment alignment"),
            ElfError::SegmentOverlapsKernel => write!(f, "Segment overlaps kernel space"),
            ElfError::NoLoadableSegments => write!(f, "No loadable segments"),
        }
    }
}

/// Information about a loaded ELF binary
#[derive(Debug, Clone)]
pub struct LoadedElf {
    /// Entry point address
    pub entry_point: u64,
    /// Base address where the ELF was loaded (for PIE)
    pub base_address: u64,
    /// Loaded segments
    pub segments: Vec<LoadedSegment>,
}

/// Information about a loaded segment
#[derive(Debug, Clone)]
pub struct LoadedSegment {
    /// Virtual address where segment is loaded
    pub vaddr: u64,
    /// Size of segment in memory
    pub memsz: u64,
    /// Size of segment in file
    pub filesz: u64,
    /// Offset of segment data in file
    pub file_offset: u64,
    /// Segment flags (readable, writable, executable)
    pub flags: u32,
}

impl LoadedSegment {
    /// Check if segment is readable
    pub fn is_readable(&self) -> bool {
        (self.flags & PF_R) != 0
    }

    /// Check if segment is writable
    pub fn is_writable(&self) -> bool {
        (self.flags & PF_W) != 0
    }

    /// Check if segment is executable
    pub fn is_executable(&self) -> bool {
        (self.flags & PF_X) != 0
    }
}

/// Parse and validate an ELF file header
///
/// # Arguments
///
/// * `data` - The ELF file data
///
/// # Returns
///
/// * `Ok(&Elf64Ehdr)` - Reference to the validated header
/// * `Err(ElfError)` - Validation error
pub fn parse_elf_header(data: &[u8]) -> Result<&Elf64Ehdr, ElfError> {
    crate::serial_println!("[ELF] parse_elf_header: start");

    // Check minimum size for ELF header
    if data.len() < size_of::<Elf64Ehdr>() {
        return Err(ElfError::FileTooSmall);
    }

    crate::serial_println!("[ELF] parse_elf_header: size check passed");

    // WORKAROUND: Manually validate using byte array indexing instead of struct deref
    // Direct struct reference is hanging for unknown reasons (PIC/relocation issue?)
    crate::serial_println!("[ELF] parse_elf_header: validating via byte array");

    // Validate magic number using array indexing
    if data[0] != 0x7F || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
        return Err(ElfError::InvalidMagic);
    }

    crate::serial_println!("[ELF] parse_elf_header: magic OK via byte check");

    // CRITICAL FIX: include_bytes! data may not be properly aligned!
    // Use read_unaligned to safely read the struct from misaligned memory
    crate::serial_println!("[ELF] parse_elf_header: using read_unaligned for safety");

    let header_ptr = data.as_ptr() as *const Elf64Ehdr;

    // WORKAROUND: Since we can't return a reference to unaligned data,
    // and read_unaligned creates a copy (wrong lifetime), we'll just
    // access fields manually through the byte array.
    // For now, create an ALIGNED copy that we can reference.

    // This is a hack but necessary: realign the data
    #[repr(C, align(8))]
    struct AlignedElfHeader {
        data: [u8; core::mem::size_of::<Elf64Ehdr>()],
    }

    static mut ALIGNED_HEADER: AlignedElfHeader = AlignedElfHeader {
        data: [0u8; core::mem::size_of::<Elf64Ehdr>()],
    };

    unsafe {
        core::ptr::copy_nonoverlapping(
            data.as_ptr(),
            ALIGNED_HEADER.data.as_mut_ptr(),
            core::mem::size_of::<Elf64Ehdr>()
        );
    }

    let header = unsafe { &*(ALIGNED_HEADER.data.as_ptr() as *const Elf64Ehdr) };
    crate::serial_println!("[ELF] parse_elf_header: header copied to aligned buffer");

    // Validate ELF magic
    if header.e_ident[0..4] != ELF_MAGIC {
        return Err(ElfError::InvalidMagic);
    }

    crate::serial_println!("[ELF] parse_elf_header: magic validated");

    // Validate ELF class (64-bit)
    if header.e_ident[4] != ELFCLASS64 {
        return Err(ElfError::InvalidClass);
    }

    // Validate endianness (little-endian)
    if header.e_ident[5] != ELFDATA2LSB {
        return Err(ElfError::InvalidEndianness);
    }

    // Validate ELF version
    if header.e_ident[6] != EV_CURRENT || header.e_version != 1 {
        return Err(ElfError::InvalidVersion);
    }

    // Validate ELF type (executable or position-independent)
    if header.e_type != ET_EXEC && header.e_type != ET_DYN {
        return Err(ElfError::InvalidType);
    }

    // Validate machine architecture (x86-64)
    if header.e_machine != EM_X86_64 {
        return Err(ElfError::InvalidMachine);
    }

    crate::serial_println!("[ELF] parse_elf_header: all validations passed");

    Ok(header)
}

/// Parse program headers from an ELF file
///
/// # Arguments
///
/// * `data` - The ELF file data
/// * `header` - The validated ELF header
///
/// # Returns
///
/// * `Ok(Vec<Elf64Phdr>)` - Vector of program headers (owned, not references)
/// * `Err(ElfError)` - Parse error
pub fn parse_program_headers(
    data: &[u8],
    header: &Elf64Ehdr,
) -> Result<Vec<Elf64Phdr>, ElfError> {
    let mut phdrs = Vec::new();

    // Calculate program header table bounds
    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;

    // Validate program header table is within file
    let phtab_size = phentsize
        .checked_mul(phnum)
        .ok_or(ElfError::ProgramHeaderOutOfBounds)?;
    let phtab_end = phoff
        .checked_add(phtab_size)
        .ok_or(ElfError::ProgramHeaderOutOfBounds)?;

    if phtab_end > data.len() {
        return Err(ElfError::ProgramHeaderOutOfBounds);
    }

    crate::serial_println!("[ELF] parse_program_headers: parsing {} headers", phnum);

    // Parse each program header
    for i in 0..phnum {
        let offset = phoff + (i * phentsize);

        // CRITICAL FIX: Use read_unaligned to handle misaligned data from include_bytes!
        // Creating a direct struct reference to misaligned memory causes hangs.
        let phdr = unsafe {
            core::ptr::read_unaligned(data.as_ptr().add(offset) as *const Elf64Phdr)
        };

        phdrs.push(phdr);
    }

    crate::serial_println!("[ELF] parse_program_headers: successfully parsed {} headers", phnum);

    Ok(phdrs)
}

/// Validate that a segment is safe to load
///
/// # Arguments
///
/// * `phdr` - Program header to validate
///
/// # Returns
///
/// * `Ok(())` - Segment is safe
/// * `Err(ElfError)` - Validation error
fn validate_segment(phdr: &Elf64Phdr) -> Result<(), ElfError> {
    // Only validate PT_LOAD segments
    if phdr.p_type != PT_LOAD {
        return Ok(());
    }

    // Check alignment is a power of 2
    if phdr.p_align > 0 && !phdr.p_align.is_power_of_two() {
        return Err(ElfError::InvalidAlignment);
    }

    // Check segment doesn't overlap with kernel space (lower half only)
    const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF;
    let end_addr = phdr
        .p_vaddr
        .checked_add(phdr.p_memsz)
        .ok_or(ElfError::SegmentOverlapsKernel)?;

    if phdr.p_vaddr >= 0xFFFF_8000_0000_0000 || end_addr > USER_SPACE_END {
        return Err(ElfError::SegmentOverlapsKernel);
    }

    Ok(())
}

/// Load an ELF file (validation only for now)
///
/// This function parses and validates an ELF file but doesn't actually
/// load it into memory yet. Full loading will be implemented when we
/// have page table support for user address spaces.
///
/// # Arguments
///
/// * `data` - The ELF file data
///
/// # Returns
///
/// * `Ok(LoadedElf)` - Information about the ELF file
/// * `Err(ElfError)` - Load error
pub fn load_elf(data: &[u8]) -> Result<LoadedElf, ElfError> {
    crate::serial_println!("[ELF] load_elf called");
    // Parse and validate ELF header
    let header = parse_elf_header(data)?;
    crate::serial_println!("[ELF] parse_elf_header returned");

    crate::serial_println!("[ELF] Parsing ELF file:");
    crate::serial_println!("[ELF]   Type: {}", if header.e_type == ET_EXEC { "ET_EXEC" } else { "ET_DYN (PIE)" });
    crate::serial_println!("[ELF]   Entry: {:#x}", header.e_entry);
    crate::serial_println!("[ELF]   Program headers: {}", header.e_phnum);

    // Parse program headers
    let phdrs = parse_program_headers(data, header)?;

    // Validate and collect loadable segments
    let mut segments = Vec::new();
    let mut has_loadable = false;

    for phdr in &phdrs {
        if phdr.p_type == PT_LOAD {
            has_loadable = true;

            // Validate segment
            validate_segment(phdr)?;

            crate::serial_println!(
                "[ELF]   Segment: vaddr={:#x} memsz={:#x} flags={}{}{}",
                phdr.p_vaddr,
                phdr.p_memsz,
                if (phdr.p_flags & PF_R) != 0 { "R" } else { "-" },
                if (phdr.p_flags & PF_W) != 0 { "W" } else { "-" },
                if (phdr.p_flags & PF_X) != 0 { "X" } else { "-" }
            );

            segments.push(LoadedSegment {
                vaddr: phdr.p_vaddr,
                memsz: phdr.p_memsz,
                filesz: phdr.p_filesz,
                file_offset: phdr.p_offset,
                flags: phdr.p_flags,
            });
        }
    }

    if !has_loadable {
        return Err(ElfError::NoLoadableSegments);
    }

    // For PIE executables, we'll need to choose a base address
    // For now, we'll use 0x400000 as a placeholder
    let base_address = if header.e_type == ET_DYN {
        0x0000_0000_0040_0000
    } else {
        0
    };

    crate::serial_println!("[ELF] ✓ ELF file validated successfully");

    Ok(LoadedElf {
        entry_point: base_address + header.e_entry,
        base_address,
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_validation() {
        // Create a minimal valid ELF header
        let mut data = [0u8; size_of::<Elf64Ehdr>()];
        data[0..4].copy_from_slice(&ELF_MAGIC);
        data[4] = ELFCLASS64;
        data[5] = ELFDATA2LSB;
        data[6] = EV_CURRENT;

        // Set e_type, e_machine, e_version (little-endian)
        data[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        data[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        data[20..24].copy_from_slice(&1u32.to_le_bytes());

        let result = parse_elf_header(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_magic() {
        let data = [0u8; size_of::<Elf64Ehdr>()];
        let result = parse_elf_header(&data);
        assert_eq!(result, Err(ElfError::InvalidMagic));
    }
}
