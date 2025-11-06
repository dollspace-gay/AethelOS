//! # Glimmer-Weave
//!
//! A scripting language for AethelOS that emphasizes natural expression,
//! query-first design, and capability-based security.
//!
//! ## Philosophy
//!
//! Glimmer-Weave is the living voice of AethelOS. Like water flowing through stone,
//! scripts are ephemeral but powerful. They exist to serve the moment, not eternity.
//!
//! ## Features
//!
//! - **Natural Expression**: Syntax that reads like intention (`bind`, `should`, `chant`)
//! - **Query-First**: Native support for World-Tree queries (`seek where...`)
//! - **Capability-Aware**: Security built into the language (`request VGA.write`)
//! - **Harmonic Failure**: Errors are suggestions, not crashes (`attempt`/`harmonize`)
//! - **Contextual Flow**: State flows naturally through pipelines (`|`)
//!
//! ## Example
//!
//! ```glimmer-weave
//! bind name to "Elara"
//! bind age to 42
//!
//! should age >= 18 then
//!     VGA.write("Welcome, " + name)
//! otherwise
//!     VGA.write("Access denied")
//! end
//! ```
//!
//! ## Modules
//!
//! - [`token`]: Token definitions for the lexer
//! - [`lexer`]: Tokenizer for Glimmer-Weave source code
//! - [`ast`]: Abstract Syntax Tree node types
//! - [`parser`]: Parser for building AST from tokens
//! - [`eval`]: Evaluator/interpreter for executing AST
//! - [`codegen`]: Code generator for compiling to x86-64 assembly

// TODO(no_std): Re-enable no_std support when glimmer_weave is integrated into kernel
// For now, use std to allow tests to run
// #![cfg_attr(not(test), no_std)]

// When not using no_std, we need to provide alloc items via std
#[cfg(not(no_std))]
extern crate std as alloc;

// When using no_std, use the real alloc crate
#[cfg(no_std)]
extern crate alloc;

pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod eval;
pub mod codegen;
pub mod elf;
pub mod runtime;
pub mod semantic;
pub mod bytecode;
pub mod bytecode_compiler;
pub mod vm;

// Re-export commonly used types
pub use token::{Token, Span};
pub use lexer::Lexer;
pub use ast::{AstNode, BinaryOperator, UnaryOperator, TypeAnnotation, Parameter};
pub use parser::{Parser, ParseError, ParseResult};
pub use eval::{Value, RuntimeError, Environment, Evaluator};
pub use codegen::{CodeGen, Instruction, Register, compile_to_asm};
pub use elf::{ElfBuilder, create_elf_object};
pub use semantic::{SemanticAnalyzer, SemanticError, Type, analyze};
