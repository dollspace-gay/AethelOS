//! Embedded test programs

/// Hello world user space program (ELF binary)
pub const HELLO_ELF: &[u8] = include_bytes!("../../target/x86_64-unknown-none/release/hello");
