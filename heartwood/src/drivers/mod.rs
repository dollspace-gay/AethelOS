//! Hardware device drivers
//!
//! This module contains drivers for various hardware devices.

pub mod ata;
pub mod serial;

pub use ata::AtaDrive;
