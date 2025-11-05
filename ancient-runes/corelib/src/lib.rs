//! # Corelib - Ancient Runes
//!
//! The standard library for AethelOS applications.
//! These are the fundamental runes that all programs use
//! to weave their purpose into existence.
//!
//! ## Philosophy
//! Corelib does not impose patterns; it provides essence.
//! These are the building blocks from which harmony emerges.

#![no_std]

extern crate alloc;

/// Common collections
pub mod collections {
    pub use alloc::vec::Vec;
    pub use alloc::string::String;
    pub use alloc::collections::BTreeMap;
    pub use alloc::collections::VecDeque;
}

/// String utilities
pub mod strings {
    /// Check if a string is empty
    pub fn is_empty(s: &str) -> bool {
        s.len() == 0
    }

    /// Count the number of graphemes (visible characters)
    pub fn grapheme_count(s: &str) -> usize {
        s.chars().count()
    }
}

/// Mathematical utilities
pub mod math {
    /// Clamp a value between min and max
    pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Linear interpolation
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Smooth step interpolation
    pub fn smoothstep(a: f32, b: f32, t: f32) -> f32 {
        let t = clamp((t - a) / (b - a), 0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

/// Result and error handling
pub mod result {
    /// A result type for AethelOS operations
    pub type AethelResult<T> = Result<T, AethelError>;

    /// Common errors in AethelOS
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AethelError {
        OutOfMemory,
        InvalidArgument,
        PermissionDenied,
        NotFound,
        AlreadyExists,
        Timeout,
        Disconnected,
    }
}

/// Syscall interface for communicating with the kernel
pub mod syscalls;

/// Re-exports for convenience
pub use collections::*;
pub use strings::*;
pub use math::*;
pub use result::*;
