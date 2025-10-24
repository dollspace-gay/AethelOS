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

/// Framebuffer tag - request a framebuffer
#[repr(C, align(8))]
pub struct FramebufferTag {
    tag_type: u16,
    flags: u16,
    size: u32,
    width: u32,
    height: u32,
    depth: u32,
}

impl FramebufferTag {
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            tag_type: TagType::Framebuffer as u16,
            flags: 0, // Optional
            size: 20, // Size of this tag
            width,
            height,
            depth,
        }
    }
}

// Calculate total header size at compile time
const HEADER_SIZE: u32 =
    core::mem::size_of::<Multiboot2Header>() as u32 +
    core::mem::size_of::<FramebufferTag>() as u32 +
    core::mem::size_of::<EndTag>() as u32;

/// The complete Multiboot2 header with tags
#[repr(C, align(8))]
pub struct CompleteHeader {
    header: Multiboot2Header,
    framebuffer: FramebufferTag,
    end: EndTag,
}

impl CompleteHeader {
    const fn new() -> Self {
        Self {
            header: Multiboot2Header::new(HEADER_SIZE),
            framebuffer: FramebufferTag::new(1024, 768, 32),
            end: EndTag::new(),
        }
    }
}

/// Place the Multiboot2 header in a special section
/// This will be placed at the start of the binary by the linker
#[used]
#[link_section = ".multiboot"]
static MULTIBOOT_HEADER: CompleteHeader = CompleteHeader::new();

// Alternative approach using inline assembly for maximum control
// This ensures the header is exactly where we want it

global_asm!(
    r#"
    .section .multiboot, "a"
    .align 8

    # Multiboot2 header
    multiboot_header_start:
        .long 0xE85250D6                  # Magic number
        .long 0                           # Architecture: i386
        .long multiboot_header_end - multiboot_header_start  # Header length
        .long -(0xE85250D6 + 0 + (multiboot_header_end - multiboot_header_start))  # Checksum

    # Framebuffer tag
    .align 8
    framebuffer_tag_start:
        .short 5                          # Type: framebuffer
        .short 0                          # Flags: optional
        .long framebuffer_tag_end - framebuffer_tag_start  # Size
        .long 1024                        # Width
        .long 768                         # Height
        .long 32                          # Depth (bits per pixel)
    framebuffer_tag_end:

    # End tag
    .align 8
        .short 0                          # Type: end
        .short 0                          # Flags
        .long 8                           # Size
    multiboot_header_end:
    "#
);

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
