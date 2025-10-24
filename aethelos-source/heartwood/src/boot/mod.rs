//! # Boot Module
//!
//! Handles the early boot process - the kernel's awakening.
//! This includes the Multiboot2 header and boot-time initialization.

pub mod multiboot2;

// Re-export for convenience
pub use multiboot2::*;
