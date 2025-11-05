//! # Code Generator - The Runic Forge
//!
//! Transforms Glimmer-Weave AST into x86-64 assembly code.
//!
//! This module implements ahead-of-time compilation, converting high-level
//! Glimmer-Weave constructs into native machine code for x86-64 processors.
//!
//! ## Architecture
//!
//! - **Calling Convention**: System V AMD64 ABI
//! - **Registers**:
//!   - Function args: rdi, rsi, rdx, rcx, r8, r9
//!   - Return value: rax
//!   - Callee-saved: rbx, r12-r15, rbp, rsp
//!   - Caller-saved: r10, r11
//! - **Stack**: 16-byte aligned before `call` instructions
//!
//! ## Output Format
//!
//! Generates AT&T syntax assembly that can be assembled with GNU as or NASM.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::ast::*;

/// x86-64 register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    Rax, Rbx, Rcx, Rdx,
    Rsi, Rdi, Rbp, Rsp,
    R8, R9, R10, R11,
    R12, R13, R14, R15,
}

impl Register {
    /// Get register name in AT&T syntax
    pub fn name(&self) -> &'static str {
        match self {
            Register::Rax => "%rax",
            Register::Rbx => "%rbx",
            Register::Rcx => "%rcx",
            Register::Rdx => "%rdx",
            Register::Rsi => "%rsi",
            Register::Rdi => "%rdi",
            Register::Rbp => "%rbp",
            Register::Rsp => "%rsp",
            Register::R8 => "%r8",
            Register::R9 => "%r9",
            Register::R10 => "%r10",
            Register::R11 => "%r11",
            Register::R12 => "%r12",
            Register::R13 => "%r13",
            Register::R14 => "%r14",
            Register::R15 => "%r15",
        }
    }
}

/// Assembly instruction
#[derive(Debug, Clone)]
pub enum Instruction {
    /// Label (for jumps, functions)
    Label(String),

    /// Move: mov src, dst
    Mov(String, String),

    /// Add: add src, dst (dst += src)
    Add(String, String),

    /// Subtract: sub src, dst (dst -= src)
    Sub(String, String),

    /// Multiply: imul src, dst (dst *= src)
    IMul(String, String),

    /// Divide: idiv divisor (rax /= divisor, rdx = remainder)
    IDiv(String),

    /// Compare: cmp op1, op2 (sets flags)
    Cmp(String, String),

    /// Jump: jmp label
    Jmp(String),

    /// Jump if equal: je label
    Je(String),

    /// Jump if not equal: jne label
    Jne(String),

    /// Jump if greater: jg label
    Jg(String),

    /// Jump if less: jl label
    Jl(String),

    /// Jump if greater or equal: jge label
    Jge(String),

    /// Jump if less or equal: jle label
    Jle(String),

    /// Call function: call label
    Call(String),

    /// Return: ret
    Ret,

    /// Push to stack: push src
    Push(String),

    /// Pop from stack: pop dst
    Pop(String),

    /// Logical AND: and src, dst
    And(String, String),

    /// Logical OR: or src, dst
    Or(String, String),

    /// Logical XOR: xor src, dst
    Xor(String, String),

    /// Logical NOT: not dst
    Not(String),

    /// Negate: neg dst
    Neg(String),

    /// Comment (for debugging generated code)
    Comment(String),
}

impl Instruction {
    /// Convert instruction to AT&T syntax assembly string
    pub fn to_asm(&self) -> String {
        match self {
            Instruction::Label(label) => format!("{}:", label),
            Instruction::Mov(src, dst) => format!("    movq {}, {}", src, dst),
            Instruction::Add(src, dst) => format!("    addq {}, {}", src, dst),
            Instruction::Sub(src, dst) => format!("    subq {}, {}", src, dst),
            Instruction::IMul(src, dst) => format!("    imulq {}, {}", src, dst),
            Instruction::IDiv(divisor) => format!("    idivq {}", divisor),
            Instruction::Cmp(op1, op2) => format!("    cmpq {}, {}", op1, op2),
            Instruction::Jmp(label) => format!("    jmp {}", label),
            Instruction::Je(label) => format!("    je {}", label),
            Instruction::Jne(label) => format!("    jne {}", label),
            Instruction::Jg(label) => format!("    jg {}", label),
            Instruction::Jl(label) => format!("    jl {}", label),
            Instruction::Jge(label) => format!("    jge {}", label),
            Instruction::Jle(label) => format!("    jle {}", label),
            Instruction::Call(label) => format!("    call {}", label),
            Instruction::Ret => "    ret".to_string(),
            Instruction::Push(src) => format!("    pushq {}", src),
            Instruction::Pop(dst) => format!("    popq {}", dst),
            Instruction::And(src, dst) => format!("    andq {}, {}", src, dst),
            Instruction::Or(src, dst) => format!("    orq {}, {}", src, dst),
            Instruction::Xor(src, dst) => format!("    xorq {}, {}", src, dst),
            Instruction::Not(dst) => format!("    notq {}", dst),
            Instruction::Neg(dst) => format!("    negq {}", dst),
            Instruction::Comment(text) => format!("    # {}", text),
        }
    }
}

/// Code generation context
pub struct CodeGen {
    /// Generated instructions
    instructions: Vec<Instruction>,

    /// Label counter (for generating unique labels)
    label_counter: usize,

    /// Current stack offset (for local variables)
    stack_offset: i32,

    /// Variable locations on stack (name -> offset from rbp)
    variables: Vec<(String, i32)>,
}

impl CodeGen {
    /// Create a new code generator
    pub fn new() -> Self {
        CodeGen {
            instructions: Vec::new(),
            label_counter: 0,
            stack_offset: 0,
            variables: Vec::new(),
        }
    }

    /// Generate a unique label
    fn gen_label(&mut self, prefix: &str) -> String {
        let label = format!(".L{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Emit an instruction
    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    /// Allocate space for a local variable
    fn alloc_var(&mut self, name: String) -> i32 {
        self.stack_offset -= 8;  // 8 bytes for i64/f64
        let offset = self.stack_offset;
        self.variables.push((name, offset));
        offset
    }

    /// Get variable stack offset
    fn get_var(&self, name: &str) -> Option<i32> {
        self.variables.iter()
            .rev()  // Search from most recent
            .find(|(n, _)| n == name)
            .map(|(_, offset)| *offset)
    }

    /// Generate code for a program (list of statements)
    pub fn compile(&mut self, nodes: &[AstNode]) -> Result<Vec<Instruction>, String> {
        // Function prologue
        self.emit(Instruction::Label("main".to_string()));
        self.emit(Instruction::Push(Register::Rbp.name().to_string()));
        self.emit(Instruction::Mov(Register::Rsp.name().to_string(), Register::Rbp.name().to_string()));

        // Generate code for each statement
        for node in nodes {
            self.gen_statement(node)?;
        }

        // Function epilogue
        self.emit(Instruction::Mov(Register::Rbp.name().to_string(), Register::Rsp.name().to_string()));
        self.emit(Instruction::Pop(Register::Rbp.name().to_string()));
        self.emit(Instruction::Ret);

        Ok(self.instructions.clone())
    }

    /// Generate code for a statement
    fn gen_statement(&mut self, node: &AstNode) -> Result<(), String> {
        match node {
            AstNode::BindStmt { name, value } | AstNode::WeaveStmt { name, value } => {
                // Evaluate expression into rax
                self.gen_expr(value)?;

                // Allocate stack space and store
                let offset = self.alloc_var(name.clone());
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", offset)
                ));

                Ok(())
            }

            AstNode::SetStmt { name, value } => {
                // Evaluate expression into rax
                self.gen_expr(value)?;

                // Store to existing variable
                let offset = self.get_var(name)
                    .ok_or_else(|| format!("Undefined variable: {}", name))?;
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", offset)
                ));

                Ok(())
            }

            AstNode::ExprStmt(expr) => {
                self.gen_expr(expr)?;
                Ok(())
            }

            _ => Err(format!("Code generation not implemented for: {:?}", node))
        }
    }

    /// Generate code for an expression (result in rax)
    fn gen_expr(&mut self, node: &AstNode) -> Result<(), String> {
        match node {
            AstNode::Number(n) => {
                // Load immediate value into rax
                self.emit(Instruction::Mov(
                    format!("${}", *n as i64),
                    Register::Rax.name().to_string()
                ));
                Ok(())
            }

            AstNode::Ident(name) => {
                // Load variable from stack into rax
                let offset = self.get_var(name)
                    .ok_or_else(|| format!("Undefined variable: {}", name))?;
                self.emit(Instruction::Mov(
                    format!("{}(%rbp)", offset),
                    Register::Rax.name().to_string()
                ));
                Ok(())
            }

            AstNode::BinaryOp { left, op, right } => {
                // Evaluate left operand into rax
                self.gen_expr(left)?;

                // Save left operand on stack
                self.emit(Instruction::Push(Register::Rax.name().to_string()));

                // Evaluate right operand into rax
                self.gen_expr(right)?;

                // Move right operand to rbx
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    Register::Rbx.name().to_string()
                ));

                // Pop left operand from stack into rax
                self.emit(Instruction::Pop(Register::Rax.name().to_string()));

                // Perform operation
                match op {
                    BinaryOperator::Add => {
                        self.emit(Instruction::Add(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                    }
                    BinaryOperator::Sub => {
                        self.emit(Instruction::Sub(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                    }
                    BinaryOperator::Mul => {
                        self.emit(Instruction::IMul(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                    }
                    BinaryOperator::Div => {
                        // For division: dividend in rax, divisor in rbx
                        // Result in rax, remainder in rdx
                        self.emit(Instruction::Xor(
                            Register::Rdx.name().to_string(),
                            Register::Rdx.name().to_string()
                        ));  // Clear rdx
                        self.emit(Instruction::IDiv(Register::Rbx.name().to_string()));
                    }
                    _ => return Err(format!("Operator not implemented: {:?}", op)),
                }

                Ok(())
            }

            AstNode::UnaryOp { op, operand } => {
                // Evaluate operand into rax
                self.gen_expr(operand)?;

                match op {
                    UnaryOperator::Negate => {
                        self.emit(Instruction::Neg(Register::Rax.name().to_string()));
                    }
                    UnaryOperator::Not => {
                        // Logical NOT: 0 -> 1, non-zero -> 0
                        self.emit(Instruction::Cmp(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov("$0".to_string(), Register::Rax.name().to_string()));
                        // TODO: Use sete to set rax based on zero flag
                    }
                }

                Ok(())
            }

            _ => Err(format!("Expression codegen not implemented: {:?}", node))
        }
    }

    /// Get generated assembly code as string
    pub fn to_assembly(&self) -> String {
        let mut asm = String::new();

        // AT&T syntax header
        asm.push_str(".text\n");
        asm.push_str(".globl main\n\n");

        for inst in &self.instructions {
            asm.push_str(&inst.to_asm());
            asm.push('\n');
        }

        asm
    }
}

/// Compile Glimmer-Weave AST to x86-64 assembly
pub fn compile_to_asm(nodes: &[AstNode]) -> Result<String, String> {
    let mut codegen = CodeGen::new();
    codegen.compile(nodes)?;
    Ok(codegen.to_assembly())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_number() {
        let ast = vec![AstNode::Number(42.0)];
        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        assert!(asm.contains("movq $42"));
    }

    #[test]
    fn test_compile_arithmetic() {
        use AstNode::*;
        use BinaryOperator::*;

        // 2 + 3
        let ast = vec![BinaryOp {
            left: Box::new(Number(2.0)),
            op: Add,
            right: Box::new(Number(3.0)),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
    }
}
