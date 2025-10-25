//! # Multiboot2 Header
//!
//! The Multiboot2 specification allows bootloaders (like GRUB) to load
//! our kernel. The header must be present in the first 32KB of the kernel
//! binary and must be 8-byte aligned.
//!
//! ## Philosophy
//! This is the handshake between bootloader and kernel - the first
//! moment of trust between two systems working together.

use core::arch::global_asm;

/// Multiboot2 magic number
/// Bootloaders look for this to identify our kernel
pub const MULTIBOOT2_MAGIC: u32 = 0xE85250D6;

/// Architecture: i386 (32-bit x86, which includes x86_64 in protected mode)
pub const MULTIBOOT2_ARCH_I386: u32 = 0;

/// Multiboot2 header structure
#[repr(C, align(8))]
pub struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // Tags follow here
}

impl Multiboot2Header {
    /// Create a new Multiboot2 header
    /// The checksum ensures magic + architecture + header_length + checksum = 0
    pub const fn new(header_length: u32) -> Self {
        let checksum = 0u32
            .wrapping_sub(MULTIBOOT2_MAGIC)
            .wrapping_sub(MULTIBOOT2_ARCH_I386)
            .wrapping_sub(header_length);

        Self {
            magic: MULTIBOOT2_MAGIC,
            architecture: MULTIBOOT2_ARCH_I386,
            header_length,
            checksum,
        }
    }
}

/// Multiboot2 header tag types
#[repr(u16)]
#[allow(dead_code)]
pub enum TagType {
    End = 0,
    InformationRequest = 1,
    Address = 2,
    EntryAddress = 3,
    Flags = 4,
    Framebuffer = 5,
    ModuleAlign = 6,
    EfiBootServices = 7,
    EfiI386EntryAddress = 8,
    EfiAmd64EntryAddress = 9,
    RelocatableHeader = 10,
}

/// Tag header common to all tags
#[repr(C, align(8))]
pub struct TagHeader {
    tag_type: u16,
    flags: u16,
    size: u32,
}

/// End tag - marks the end of the header
#[repr(C, align(8))]
pub struct EndTag {
    tag_type: u16,
    flags: u16,
    size: u32,
}

impl EndTag {
    pub const fn new() -> Self {
        Self {
            tag_type: TagType::End as u16,
            flags: 0,
            size: 8, // Size of this tag
        }
    }
}

/// Entry address tag - tells bootloader where to jump
#[repr(C, align(8))]
pub struct EntryAddressTag {
    tag_type: u16,
    flags: u16,
    size: u32,
    entry_addr: u32,
}

impl EntryAddressTag {
    pub const fn new(entry_addr: u32) -> Self {
        Self {
            tag_type: TagType::EntryAddress as u16,
            flags: 0,
            size: 12, // Size of this tag (8 bytes header + 4 bytes address)
            entry_addr,
        }
    }
}

/// Console Flags Tag - request EGA text console
#[repr(C, align(8))]
pub struct ConsoleTag {
    tag_type: u16,
    flags: u16,
    size: u32,
    console_flags: u32,
}

impl ConsoleTag {
    pub const fn new() -> Self {
        Self {
            tag_type: 4,  // Console flags tag type
            flags: 0,     // Optional request
            size: 12,     // Size of this tag
            console_flags: 3,  // EGA_TEXT_SUPPORTED (bit 0) + REQUIRE (bit 1)
        }
    }
}

// Calculate total header size at compile time
const HEADER_SIZE: u32 =
    core::mem::size_of::<Multiboot2Header>() as u32 +
    core::mem::size_of::<ConsoleTag>() as u32 +
    core::mem::size_of::<EndTag>() as u32;

/// The complete Multiboot2 header with tags
#[repr(C, align(8))]
pub struct CompleteHeader {
    header: Multiboot2Header,
    console: ConsoleTag,  // The Rune of Console Preference
    end: EndTag,
}

impl CompleteHeader {
    const fn new() -> Self {
        // Request EGA text console explicitly via Console Flags Tag
        Self {
            header: Multiboot2Header::new(HEADER_SIZE),
            console: ConsoleTag::new(),
            end: EndTag::new(),
        }
    }
}

/// Place the Multiboot2 header with Console Flags Tag in a special section
/// This will be placed at the start of the binary by the linker
/// The Console Flags Tag is the Rune of Console Preference
#[used]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: CompleteHeader = CompleteHeader::new();

// NOTE: The assembly approach with global_asm! doesn't work reliably for
// creating custom sections in Rust. We use the Rust struct above instead.

/// Verify the header is valid at compile time
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(
            core::mem::size_of::<CompleteHeader>(),
            HEADER_SIZE as usize
        );
    }

    #[test]
    fn test_checksum() {
        let header = Multiboot2Header::new(HEADER_SIZE);
        let sum = header.magic
            .wrapping_add(header.architecture)
            .wrapping_add(header.header_length)
            .wrapping_add(header.checksum);
        assert_eq!(sum, 0, "Checksum must make sum equal to zero");
    }

    #[test]
    fn test_alignment() {
        assert_eq!(
            core::mem::align_of::<CompleteHeader>(),
            8,
            "Header must be 8-byte aligned"
        );
    }

    #[test]
    fn test_magic() {
        let header = Multiboot2Header::new(HEADER_SIZE);
        assert_eq!(header.magic, MULTIBOOT2_MAGIC);
    }
}
