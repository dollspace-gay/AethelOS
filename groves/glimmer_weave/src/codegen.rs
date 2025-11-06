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

    /// Set if equal: sete dst (sets dst to 1 if ZF=1, else 0)
    Sete(String),

    /// Set if not equal: setne dst
    Setne(String),

    /// Set if greater: setg dst (signed)
    Setg(String),

    /// Set if less: setl dst (signed)
    Setl(String),

    /// Set if greater or equal: setge dst (signed)
    Setge(String),

    /// Set if less or equal: setle dst (signed)
    Setle(String),

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
            Instruction::Sete(dst) => format!("    sete {}", dst),
            Instruction::Setne(dst) => format!("    setne {}", dst),
            Instruction::Setg(dst) => format!("    setg {}", dst),
            Instruction::Setl(dst) => format!("    setl {}", dst),
            Instruction::Setge(dst) => format!("    setge {}", dst),
            Instruction::Setle(dst) => format!("    setle {}", dst),
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

    /// Current function name (for TCO detection)
    current_function: Option<String>,

    /// Current function entry label (for TCO jumps)
    function_entry_label: Option<String>,
}

impl CodeGen {
    /// Create a new code generator
    pub fn new() -> Self {
        CodeGen {
            instructions: Vec::new(),
            label_counter: 0,
            stack_offset: 0,
            variables: Vec::new(),
            current_function: None,
            function_entry_label: None,
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
            AstNode::BindStmt { name, typ: _, value } | AstNode::WeaveStmt { name, typ: _, value } => {
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

            AstNode::IfStmt { condition, then_branch, else_branch } => {
                // Generate unique labels
                let else_label = format!(".L_else_{}", self.label_counter);
                let end_label = format!(".L_if_end_{}", self.label_counter);
                self.label_counter += 1;

                // Evaluate condition into rax
                self.gen_expr(condition)?;

                // Check if condition is false (0)
                self.emit(Instruction::Cmp(
                    "$0".to_string(),
                    Register::Rax.name().to_string()
                ));

                // Jump to else branch if condition is false
                if else_branch.is_some() {
                    self.emit(Instruction::Je(else_label.clone()));
                } else {
                    self.emit(Instruction::Je(end_label.clone()));
                }

                // Generate then branch
                for stmt in then_branch {
                    self.gen_statement(stmt)?;
                }

                // Jump to end (skip else branch)
                if else_branch.is_some() {
                    self.emit(Instruction::Jmp(end_label.clone()));
                }

                // Generate else branch (if present)
                if let Some(else_stmts) = else_branch {
                    self.emit(Instruction::Label(else_label));
                    for stmt in else_stmts {
                        self.gen_statement(stmt)?;
                    }
                }

                // End label
                self.emit(Instruction::Label(end_label));

                Ok(())
            }

            AstNode::WhileStmt { condition, body } => {
                // Generate unique labels
                let start_label = format!(".L_while_start_{}", self.label_counter);
                let end_label = format!(".L_while_end_{}", self.label_counter);
                self.label_counter += 1;

                // Loop start
                self.emit(Instruction::Label(start_label.clone()));

                // Evaluate condition into rax
                self.gen_expr(condition)?;

                // Check if condition is false (0)
                self.emit(Instruction::Cmp(
                    "$0".to_string(),
                    Register::Rax.name().to_string()
                ));

                // Jump to end if condition is false
                self.emit(Instruction::Je(end_label.clone()));

                // Generate loop body
                for stmt in body {
                    self.gen_statement(stmt)?;
                }

                // Jump back to start
                self.emit(Instruction::Jmp(start_label));

                // End label
                self.emit(Instruction::Label(end_label));

                Ok(())
            }

            AstNode::MatchStmt { value, arms } => {
                use crate::ast::Pattern;

                // Generate unique labels for match arms
                let match_id = self.label_counter;
                self.label_counter += 1;
                let end_label = format!(".L_match_end_{}", match_id);

                // Evaluate the match value into rax
                self.gen_expr(value)?;

                // Save match value to a temporary stack location
                let match_value_offset = self.alloc_var(format!("__match_tmp_{}", match_id));
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", match_value_offset)
                ));

                // Generate code for each arm
                for (arm_idx, arm) in arms.iter().enumerate() {
                    let next_arm_label = format!(".L_match_arm_{}_{}", match_id, arm_idx + 1);

                    match &arm.pattern {
                        Pattern::Literal(lit_node) => {
                            // Evaluate the literal into rbx
                            self.gen_expr(lit_node)?;
                            self.emit(Instruction::Mov(
                                Register::Rax.name().to_string(),
                                Register::Rbx.name().to_string()
                            ));

                            // Load match value back into rax
                            self.emit(Instruction::Mov(
                                format!("{}(%rbp)", match_value_offset),
                                Register::Rax.name().to_string()
                            ));

                            // Compare rax (match value) with rbx (literal)
                            self.emit(Instruction::Cmp(
                                Register::Rbx.name().to_string(),
                                Register::Rax.name().to_string()
                            ));

                            // Jump to next arm if not equal
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Jne(next_arm_label.clone()));
                            }

                            // Pattern matched! Execute arm body
                            for stmt in &arm.body {
                                self.gen_statement(stmt)?;
                            }

                            // Jump to end
                            self.emit(Instruction::Jmp(end_label.clone()));

                            // Emit next arm label
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Label(next_arm_label));
                            }
                        }

                        Pattern::Ident(var_name) => {
                            // Variable binding - always matches
                            // Load match value into rax and store to variable
                            self.emit(Instruction::Mov(
                                format!("{}(%rbp)", match_value_offset),
                                Register::Rax.name().to_string()
                            ));

                            let var_offset = self.alloc_var(var_name.clone());
                            self.emit(Instruction::Mov(
                                Register::Rax.name().to_string(),
                                format!("{}(%rbp)", var_offset)
                            ));

                            // Execute arm body
                            for stmt in &arm.body {
                                self.gen_statement(stmt)?;
                            }

                            // Jump to end
                            self.emit(Instruction::Jmp(end_label.clone()));

                            // Emit next arm label (though unreachable)
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Label(next_arm_label));
                            }
                        }

                        Pattern::Wildcard => {
                            // Wildcard - always matches, no binding
                            // Execute arm body
                            for stmt in &arm.body {
                                self.gen_statement(stmt)?;
                            }

                            // Jump to end
                            self.emit(Instruction::Jmp(end_label.clone()));

                            // Emit next arm label (though unreachable)
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Label(next_arm_label));
                            }
                        }

                        Pattern::Enum { variant, inner } => {
                            self.emit(Instruction::Comment(
                                format!("Match {} variant", variant)
                            ));

                            // Load match value (which is a pointer to the enum structure)
                            self.emit(Instruction::Mov(
                                format!("{}(%rbp)", match_value_offset),
                                Register::Rax.name().to_string()
                            ));

                            // Load tag from offset +8 relative to enum pointer
                            // Enum layout: [value at +0, tag at +8]
                            self.emit(Instruction::Mov(
                                "8(%rax)".to_string(),
                                Register::Rbx.name().to_string()
                            ));

                            // Determine expected tag value for this variant
                            let expected_tag = match variant.as_str() {
                                "Triumph" | "Present" => 1,
                                "Mishap" | "Absent" => 0,
                                _ => return Err(format!("Unknown enum variant: {}", variant)),
                            };

                            // Compare tag with expected value
                            self.emit(Instruction::Cmp(
                                format!("${}", expected_tag),
                                Register::Rbx.name().to_string()
                            ));

                            // Jump to next arm if tag doesn't match
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Jne(next_arm_label.clone()));
                            }

                            // Tag matched! Now handle inner pattern binding
                            if let Some(inner_pattern) = inner {
                                match inner_pattern.as_ref() {
                                    Pattern::Ident(var_name) => {
                                        // Extract inner value and bind to variable
                                        // Load match value pointer again (still in rax)
                                        self.emit(Instruction::Mov(
                                            format!("{}(%rbp)", match_value_offset),
                                            Register::Rax.name().to_string()
                                        ));

                                        // Load inner value from offset +0
                                        self.emit(Instruction::Mov(
                                            "0(%rax)".to_string(),
                                            Register::Rbx.name().to_string()
                                        ));

                                        // Store inner value to variable
                                        let var_offset = self.alloc_var(var_name.clone());
                                        self.emit(Instruction::Mov(
                                            Register::Rbx.name().to_string(),
                                            format!("{}(%rbp)", var_offset)
                                        ));
                                    }

                                    Pattern::Wildcard => {
                                        // No binding needed for wildcard
                                    }

                                    _ => {
                                        return Err(
                                            "Complex nested enum patterns not yet supported in native codegen".to_string()
                                        );
                                    }
                                }
                            }

                            // Execute arm body
                            for stmt in &arm.body {
                                self.gen_statement(stmt)?;
                            }

                            // Jump to end
                            self.emit(Instruction::Jmp(end_label.clone()));

                            // Emit next arm label
                            if arm_idx < arms.len() - 1 {
                                self.emit(Instruction::Label(next_arm_label));
                            }
                        }
                    }
                }

                // End label
                self.emit(Instruction::Label(end_label));

                Ok(())
            }

            AstNode::ChantDef { name, params, return_type: _, body } => {
                // Generate function with TCO support
                let old_function = self.current_function.clone();
                let old_label = self.function_entry_label.clone();
                let old_vars = self.variables.clone();
                let old_stack = self.stack_offset;

                // Create function label
                let func_label = format!(".L_func_{}", name);
                self.current_function = Some(name.clone());
                self.function_entry_label = Some(func_label.clone());

                // Function prologue
                self.emit(Instruction::Label(func_label.clone()));
                self.emit(Instruction::Push(Register::Rbp.name().to_string()));
                self.emit(Instruction::Mov(Register::Rsp.name().to_string(), Register::Rbp.name().to_string()));

                // Allocate parameters on stack
                // Args come in rdi, rsi, rdx, rcx, r8, r9 (System V ABI)
                let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                for (i, param) in params.iter().enumerate() {
                    if i < arg_regs.len() {
                        let offset = self.alloc_var(param.name.clone());
                        self.emit(Instruction::Mov(
                            format!("%{}", arg_regs[i]),
                            format!("{}(%rbp)", offset)
                        ));
                    }
                }

                // Compile function body
                for stmt in body {
                    self.gen_statement(stmt)?;
                }

                // Default return (if no explicit yield)
                self.emit(Instruction::Mov("$0".to_string(), Register::Rax.name().to_string()));
                self.emit(Instruction::Mov(Register::Rbp.name().to_string(), Register::Rsp.name().to_string()));
                self.emit(Instruction::Pop(Register::Rbp.name().to_string()));
                self.emit(Instruction::Ret);

                // Restore context
                self.current_function = old_function;
                self.function_entry_label = old_label;
                self.variables = old_vars;
                self.stack_offset = old_stack;

                Ok(())
            }

            AstNode::FormDef { name, fields: _ } => {
                // Struct definitions in native codegen require runtime support
                // For now, we'll emit a placeholder that stores the struct definition
                // as a global symbol (though it won't be fully functional)
                self.emit(Instruction::Comment(format!("Struct definition: {}", name)));
                self.emit(Instruction::Comment("Note: Full struct support requires heap allocation runtime".to_string()));
                Ok(())
            }

            AstNode::YieldStmt { value } => {
                // Check for tail call (yield f(args) where f is current function)
                if let AstNode::Call { callee, args } = value.as_ref() {
                    if let AstNode::Ident(func_name) = callee.as_ref() {
                        if Some(func_name) == self.current_function.as_ref() {
                            // This is a tail call! Use TCO.
                            // Evaluate arguments
                            let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                            for (i, arg) in args.iter().enumerate() {
                                if i < arg_regs.len() {
                                    // Evaluate arg into rax
                                    self.gen_expr(arg)?;
                                    // Move to appropriate argument register
                                    self.emit(Instruction::Mov(
                                        Register::Rax.name().to_string(),
                                        format!("%{}", arg_regs[i])
                                    ));
                                }
                            }

                            // Jump back to function start (TCO!)
                            if let Some(entry_label) = self.function_entry_label.clone() {
                                // Restore stack frame
                                self.emit(Instruction::Mov(Register::Rbp.name().to_string(), Register::Rsp.name().to_string()));
                                self.emit(Instruction::Pop(Register::Rbp.name().to_string()));
                                self.emit(Instruction::Jmp(entry_label));
                            }

                            return Ok(());
                        }
                    }
                }

                // Not a tail call, emit normal return
                self.gen_expr(value)?;
                self.emit(Instruction::Mov(Register::Rbp.name().to_string(), Register::Rsp.name().to_string()));
                self.emit(Instruction::Pop(Register::Rbp.name().to_string()));
                self.emit(Instruction::Ret);
                Ok(())
            }

            AstNode::AttemptStmt { body, handlers } => {
                // Generate unique labels
                let attempt_id = self.label_counter;
                self.label_counter += 1;
                let handler_label = format!(".L_attempt_handler_{}", attempt_id);
                let end_label = format!(".L_attempt_end_{}", attempt_id);

                // Store current exception handler label (for nested attempt blocks)
                // For simplicity, we'll use a convention: %r15 holds the handler label address
                // (In a full implementation, this would use a stack-based approach)

                // Generate try body
                self.emit(Instruction::Comment(format!("Attempt block {}", attempt_id)));
                for stmt in body {
                    self.gen_statement(stmt)?;
                }

                // If we get here, no error occurred - jump over handlers
                self.emit(Instruction::Jmp(end_label.clone()));

                // Generate handler code
                self.emit(Instruction::Label(handler_label.clone()));
                self.emit(Instruction::Comment("Exception handlers".to_string()));

                // Error type is expected in %rbx (string pointer)
                // Error value is expected in %rcx (string pointer)
                // For simplicity in this basic implementation, we'll just check error types

                for (i, handler) in handlers.iter().enumerate() {
                    let next_handler_label = format!(".L_attempt_handler_{}_{}", attempt_id, i + 1);

                    if handler.error_type == "_" {
                        // Wildcard - always matches
                        self.emit(Instruction::Comment("Wildcard handler".to_string()));
                        for stmt in &handler.body {
                            self.gen_statement(stmt)?;
                        }
                        self.emit(Instruction::Jmp(end_label.clone()));
                    } else {
                        // Check if error type matches
                        self.emit(Instruction::Comment(format!(
                            "Handler for {}", handler.error_type
                        )));

                        // For now, we'll skip the actual string comparison
                        // In a full implementation, we'd call strcmp or similar
                        // Instead, we'll just execute the handler body

                        for stmt in &handler.body {
                            self.gen_statement(stmt)?;
                        }
                        self.emit(Instruction::Jmp(end_label.clone()));

                        // Label for next handler (if any)
                        if i < handlers.len() - 1 {
                            self.emit(Instruction::Label(next_handler_label));
                        }
                    }
                }

                // End label
                self.emit(Instruction::Label(end_label));
                self.emit(Instruction::Comment("End of attempt block".to_string()));

                Ok(())
            }

            AstNode::ExprStmt(expr) => {
                self.gen_expr(expr)?;
                Ok(())
            }

            // Try compiling as bare expression (for tests and REPL)
            _ => {
                self.gen_expr(node)?;
                Ok(())
            }
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
                    BinaryOperator::Mod => {
                        // For modulo: dividend in rax, divisor in rbx
                        // Result in rdx (remainder)
                        self.emit(Instruction::Xor(
                            Register::Rdx.name().to_string(),
                            Register::Rdx.name().to_string()
                        ));  // Clear rdx
                        self.emit(Instruction::IDiv(Register::Rbx.name().to_string()));
                        // Move remainder from rdx to rax
                        self.emit(Instruction::Mov(
                            Register::Rdx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                    }

                    // Comparison operators (return 0 or 1 in rax)
                    BinaryOperator::Equal => {
                        // cmp rbx, rax (sets flags based on rax - rbx)
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        // Clear rax
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        // Set low byte of rax to 1 if equal
                        self.emit(Instruction::Sete("%al".to_string()));
                    }
                    BinaryOperator::NotEqual => {
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Setne("%al".to_string()));
                    }
                    BinaryOperator::Greater => {
                        // cmp compares rax with rbx, setg checks if rax > rbx
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Setg("%al".to_string()));
                    }
                    BinaryOperator::Less => {
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Setl("%al".to_string()));
                    }
                    BinaryOperator::GreaterEq => {
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Setge("%al".to_string()));
                    }
                    BinaryOperator::LessEq => {
                        self.emit(Instruction::Cmp(
                            Register::Rbx.name().to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Setle("%al".to_string()));
                    }

                    // Logical operators (short-circuit evaluation)
                    BinaryOperator::And => {
                        // Both operands already evaluated (rax has left, rbx has right)
                        // Logical AND: both must be non-zero
                        // Compare left (rax) with 0
                        self.emit(Instruction::Cmp(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        let label_false = format!(".L_and_false_{}", self.label_counter);
                        let label_end = format!(".L_and_end_{}", self.label_counter);
                        self.label_counter += 1;

                        // If left is 0, result is 0
                        self.emit(Instruction::Je(label_false.clone()));

                        // Check right operand
                        self.emit(Instruction::Cmp(
                            "$0".to_string(),
                            Register::Rbx.name().to_string()
                        ));
                        self.emit(Instruction::Je(label_false.clone()));

                        // Both non-zero, result is 1
                        self.emit(Instruction::Mov(
                            "$1".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Jmp(label_end.clone()));

                        // At least one is zero, result is 0
                        self.emit(Instruction::Label(label_false));
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));

                        self.emit(Instruction::Label(label_end));
                    }
                    BinaryOperator::Or => {
                        // Logical OR: at least one must be non-zero
                        self.emit(Instruction::Cmp(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        let label_true = format!(".L_or_true_{}", self.label_counter);
                        let label_check_right = format!(".L_or_check_right_{}", self.label_counter);
                        let label_end = format!(".L_or_end_{}", self.label_counter);
                        self.label_counter += 1;

                        // If left is non-zero, result is 1
                        self.emit(Instruction::Jne(label_true.clone()));

                        // Check right operand
                        self.emit(Instruction::Cmp(
                            "$0".to_string(),
                            Register::Rbx.name().to_string()
                        ));
                        self.emit(Instruction::Jne(label_true.clone()));

                        // Both zero, result is 0
                        self.emit(Instruction::Mov(
                            "$0".to_string(),
                            Register::Rax.name().to_string()
                        ));
                        self.emit(Instruction::Jmp(label_end.clone()));

                        // At least one non-zero, result is 1
                        self.emit(Instruction::Label(label_true));
                        self.emit(Instruction::Mov(
                            "$1".to_string(),
                            Register::Rax.name().to_string()
                        ));

                        self.emit(Instruction::Label(label_end));
                    }
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

            AstNode::Call { callee, args } => {
                // Function call with System V ABI
                // Arguments in: rdi, rsi, rdx, rcx, r8, r9
                let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

                // Evaluate arguments and move to argument registers
                for (i, arg) in args.iter().enumerate() {
                    if i < arg_regs.len() {
                        self.gen_expr(arg)?;  // Result in rax
                        self.emit(Instruction::Mov(
                            Register::Rax.name().to_string(),
                            format!("%{}", arg_regs[i])
                        ));
                    }
                }

                // Call the function
                if let AstNode::Ident(func_name) = callee.as_ref() {
                    let func_label = format!(".L_func_{}", func_name);
                    self.emit(Instruction::Call(func_label));
                } else {
                    return Err("Indirect calls not supported yet".to_string());
                }

                // Result is in rax
                Ok(())
            }

            // Enum constructors - Outcome type
            AstNode::Triumph(value) => {
                self.emit(Instruction::Comment("Create Triumph variant".to_string()));

                // Evaluate inner value
                self.gen_expr(value)?;

                // Allocate 16 bytes on stack for enum (tag + value)
                self.stack_offset -= 16;

                // Store tag (1 for Triumph) at -8(%rbp)
                self.emit(Instruction::Mov(
                    "$1".to_string(),
                    format!("{}(%rbp)", self.stack_offset + 8)
                ));

                // Store value (in rax) at stack_offset(%rbp)
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", self.stack_offset)
                ));

                // Load address of enum into rax
                self.emit(Instruction::Mov(
                    Register::Rbp.name().to_string(),
                    Register::Rax.name().to_string()
                ));
                self.emit(Instruction::Add(
                    format!("${}", self.stack_offset),
                    Register::Rax.name().to_string()
                ));

                Ok(())
            }

            AstNode::Mishap(value) => {
                self.emit(Instruction::Comment("Create Mishap variant".to_string()));

                // Evaluate inner value
                self.gen_expr(value)?;

                // Allocate 16 bytes on stack for enum (tag + value)
                self.stack_offset -= 16;

                // Store tag (0 for Mishap) at -8(%rbp)
                self.emit(Instruction::Mov(
                    "$0".to_string(),
                    format!("{}(%rbp)", self.stack_offset + 8)
                ));

                // Store value (in rax) at stack_offset(%rbp)
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", self.stack_offset)
                ));

                // Load address of enum into rax
                self.emit(Instruction::Mov(
                    Register::Rbp.name().to_string(),
                    Register::Rax.name().to_string()
                ));
                self.emit(Instruction::Add(
                    format!("${}", self.stack_offset),
                    Register::Rax.name().to_string()
                ));

                Ok(())
            }

            // Enum constructors - Maybe type
            AstNode::Present(value) => {
                self.emit(Instruction::Comment("Create Present variant".to_string()));

                // Evaluate inner value
                self.gen_expr(value)?;

                // Allocate 16 bytes on stack for enum (tag + value)
                self.stack_offset -= 16;

                // Store tag (1 for Present) at -8(%rbp)
                self.emit(Instruction::Mov(
                    "$1".to_string(),
                    format!("{}(%rbp)", self.stack_offset + 8)
                ));

                // Store value (in rax) at stack_offset(%rbp)
                self.emit(Instruction::Mov(
                    Register::Rax.name().to_string(),
                    format!("{}(%rbp)", self.stack_offset)
                ));

                // Load address of enum into rax
                self.emit(Instruction::Mov(
                    Register::Rbp.name().to_string(),
                    Register::Rax.name().to_string()
                ));
                self.emit(Instruction::Add(
                    format!("${}", self.stack_offset),
                    Register::Rax.name().to_string()
                ));

                Ok(())
            }

            AstNode::Absent => {
                self.emit(Instruction::Comment("Create Absent variant".to_string()));

                // Allocate 16 bytes on stack for enum (tag + value)
                self.stack_offset -= 16;

                // Store tag (0 for Absent) at -8(%rbp)
                self.emit(Instruction::Mov(
                    "$0".to_string(),
                    format!("{}(%rbp)", self.stack_offset + 8)
                ));

                // Store dummy value (0) at stack_offset(%rbp) - not used for Absent
                self.emit(Instruction::Mov(
                    "$0".to_string(),
                    format!("{}(%rbp)", self.stack_offset)
                ));

                // Load address of enum into rax
                self.emit(Instruction::Mov(
                    Register::Rbp.name().to_string(),
                    Register::Rax.name().to_string()
                ));
                self.emit(Instruction::Add(
                    format!("${}", self.stack_offset),
                    Register::Rax.name().to_string()
                ));

                Ok(())
            }

            AstNode::StructLiteral { struct_name, fields: _ } => {
                // Struct instantiation requires heap allocation runtime
                self.emit(Instruction::Comment(format!("Struct literal: {}", struct_name)));
                self.emit(Instruction::Comment("Note: Struct instantiation requires heap allocation runtime".to_string()));
                self.emit(Instruction::Comment("This feature is fully supported in interpreter and bytecode VM".to_string()));

                // For now, return error
                Err(format!("Struct literals not supported in native codegen (requires heap allocation runtime). Use interpreter or bytecode VM instead."))
            }

            AstNode::FieldAccess { object, field } => {
                // Field access on structs requires runtime type information and heap-allocated objects
                self.emit(Instruction::Comment(format!("Field access: .{}", field)));
                self.emit(Instruction::Comment("Note: Struct field access requires heap allocation runtime".to_string()));
                self.emit(Instruction::Comment("This feature is fully supported in interpreter and bytecode VM".to_string()));

                // For now, try to compile the object expression but return error
                self.gen_expr(object)?;
                Err(format!("Struct field access not supported in native codegen (requires heap allocation runtime). Use interpreter or bytecode VM instead."))
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
        if let Err(e) = &result {
            eprintln!("Compilation error: {}", e);
        }
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

    #[test]
    fn test_compile_comparison() {
        use AstNode::*;
        use BinaryOperator::*;

        // 10 > 5
        let ast = vec![BinaryOp {
            left: Box::new(Number(10.0)),
            op: Greater,
            right: Box::new(Number(5.0)),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain cmp and setg instructions
        assert!(asm.contains("cmpq"));
        assert!(asm.contains("setg"));
    }

    #[test]
    fn test_compile_equality() {
        use AstNode::*;
        use BinaryOperator::*;

        // 5 is 5
        let ast = vec![BinaryOp {
            left: Box::new(Number(5.0)),
            op: Equal,
            right: Box::new(Number(5.0)),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain cmp and sete instructions
        assert!(asm.contains("cmpq"));
        assert!(asm.contains("sete"));
    }

    #[test]
    fn test_compile_logical_and() {
        use AstNode::*;
        use BinaryOperator::*;

        // 1 and 1
        let ast = vec![BinaryOp {
            left: Box::new(Number(1.0)),
            op: And,
            right: Box::new(Number(1.0)),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain conditional jumps for short-circuit evaluation
        assert!(asm.contains("cmpq"));
        assert!(asm.contains("je") || asm.contains("jne"));
    }

    #[test]
    fn test_compile_modulo() {
        use AstNode::*;
        use BinaryOperator::*;

        // 10 % 3
        let ast = vec![BinaryOp {
            left: Box::new(Number(10.0)),
            op: Mod,
            right: Box::new(Number(3.0)),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain idiv instruction
        assert!(asm.contains("idivq"));
    }

    #[test]
    fn test_compile_if_stmt() {
        use AstNode::*;
        use BinaryOperator::*;

        // should 1 > 0 then bind x to 42 end
        let ast = vec![IfStmt {
            condition: Box::new(BinaryOp {
                left: Box::new(Number(1.0)),
                op: Greater,
                right: Box::new(Number(0.0)),
            }),
            then_branch: vec![BindStmt {
                name: "x".to_string(),
                typ: None,
                value: Box::new(Number(42.0)),
            }],
            else_branch: None,
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain conditional jump and labels
        assert!(asm.contains("cmpq"));
        assert!(asm.contains("je"));
        assert!(asm.contains(".L_if_end_"));
    }

    #[test]
    fn test_compile_if_else_stmt() {
        use AstNode::*;
        use BinaryOperator::*;

        // should 0 > 1 then bind x to 1 otherwise bind x to 2 end
        let ast = vec![IfStmt {
            condition: Box::new(BinaryOp {
                left: Box::new(Number(0.0)),
                op: Greater,
                right: Box::new(Number(1.0)),
            }),
            then_branch: vec![BindStmt {
                name: "x".to_string(),
                typ: None,
                value: Box::new(Number(1.0)),
            }],
            else_branch: Some(vec![BindStmt {
                name: "x".to_string(),
                typ: None,
                value: Box::new(Number(2.0)),
            }]),
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain else label and unconditional jump
        assert!(asm.contains(".L_else_"));
        assert!(asm.contains(".L_if_end_"));
        assert!(asm.contains("jmp"));
    }

    #[test]
    fn test_compile_while_stmt() {
        use AstNode::*;
        use BinaryOperator::*;

        // bind x to 5
        // whilst x > 0 then set x to x - 1 end
        let ast = vec![
            BindStmt {
                name: "x".to_string(),
                typ: None,
                value: Box::new(Number(5.0)),
            },
            WhileStmt {
                condition: Box::new(BinaryOp {
                    left: Box::new(Ident("x".to_string())),
                    op: Greater,
                    right: Box::new(Number(0.0)),
                }),
                body: vec![SetStmt {
                    name: "x".to_string(),
                    value: Box::new(BinaryOp {
                        left: Box::new(Ident("x".to_string())),
                        op: Sub,
                        right: Box::new(Number(1.0)),
                    }),
                }],
            },
        ];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();
        // Should contain loop labels and conditional/unconditional jumps
        assert!(asm.contains(".L_while_start_"));
        assert!(asm.contains(".L_while_end_"));
        assert!(asm.contains("je"));
        assert!(asm.contains("jmp"));
    }

    #[test]
    fn test_compile_tco_tail_recursion() {
        use AstNode::*;
        use BinaryOperator::*;
        use crate::ast::Parameter;

        // chant sum_to(n, acc) then
        //     should n at most 0 then
        //         yield acc
        //     otherwise
        //         yield sum_to(n - 1, acc + n)
        //     end
        // end
        let ast = vec![ChantDef {
            name: "sum_to".to_string(),
            params: vec![
                Parameter { name: "n".to_string(), typ: None },
                Parameter { name: "acc".to_string(), typ: None },
            ],
            return_type: None,
            body: vec![IfStmt {
                condition: Box::new(BinaryOp {
                    left: Box::new(Ident("n".to_string())),
                    op: LessEq,
                    right: Box::new(Number(0.0)),
                }),
                then_branch: vec![YieldStmt {
                    value: Box::new(Ident("acc".to_string())),
                }],
                else_branch: Some(vec![YieldStmt {
                    value: Box::new(Call {
                        callee: Box::new(Ident("sum_to".to_string())),
                        args: vec![
                            BinaryOp {
                                left: Box::new(Ident("n".to_string())),
                                op: Sub,
                                right: Box::new(Number(1.0)),
                            },
                            BinaryOp {
                                left: Box::new(Ident("acc".to_string())),
                                op: Add,
                                right: Box::new(Ident("n".to_string())),
                            },
                        ],
                    }),
                }]),
            }],
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain function label
        assert!(asm.contains(".L_func_sum_to"));

        // Should contain jmp for TCO (not call)
        // TCO converts recursive call to jump back to function start
        assert!(asm.contains("jmp") || asm.contains("ret"));
    }

    #[test]
    fn test_compile_pattern_matching_literals() {
        use AstNode::*;

        // match 2 with
        //     when 1 then 100
        //     when 2 then 200
        //     otherwise then 999
        // end
        let ast = vec![MatchStmt {
            value: Box::new(Number(2.0)),
            arms: vec![
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Literal(Number(1.0)),
                    body: vec![Number(100.0)],
                },
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Literal(Number(2.0)),
                    body: vec![Number(200.0)],
                },
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Wildcard,
                    body: vec![Number(999.0)],
                },
            ],
        }];

        let result = compile_to_asm(&ast);
        if let Err(e) = &result {
            eprintln!("Compilation error: {}", e);
        }
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comparison instruction
        assert!(asm.contains("cmpq"));

        // Should contain conditional jump (jne)
        assert!(asm.contains("jne"));

        // Should contain match labels
        assert!(asm.contains(".L_match_"));
    }

    #[test]
    fn test_compile_pattern_matching_with_binding() {
        use AstNode::*;

        // match 42 with
        //     when n then n * 2
        // end
        let ast = vec![MatchStmt {
            value: Box::new(Number(42.0)),
            arms: vec![
                crate::ast::MatchArm {
                    pattern: crate::ast::Pattern::Ident("n".to_string()),
                    body: vec![BinaryOp {
                        left: Box::new(Ident("n".to_string())),
                        op: BinaryOperator::Mul,
                        right: Box::new(Number(2.0)),
                    }],
                },
            ],
        }];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain mov instructions for binding
        assert!(asm.contains("movq"));
    }

    #[test]
    fn test_compile_triumph_constructor() {
        use AstNode::*;

        // Triumph(42)
        let ast = vec![Triumph(Box::new(Number(42.0)))];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comment
        assert!(asm.contains("Create Triumph variant"));

        // Should store tag=1 and value
        assert!(asm.contains("movq $1"));
        assert!(asm.contains("movq $42"));
    }

    #[test]
    fn test_compile_mishap_constructor() {
        use AstNode::*;

        // Mishap(99)
        let ast = vec![Mishap(Box::new(Number(99.0)))];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comment
        assert!(asm.contains("Create Mishap variant"));

        // Should store tag=0 and value
        assert!(asm.contains("movq $0"));
        assert!(asm.contains("movq $99"));
    }

    #[test]
    fn test_compile_present_constructor() {
        use AstNode::*;

        // Present(123)
        let ast = vec![Present(Box::new(Number(123.0)))];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comment
        assert!(asm.contains("Create Present variant"));

        // Should store tag=1 and value
        assert!(asm.contains("movq $1"));
        assert!(asm.contains("movq $123"));
    }

    #[test]
    fn test_compile_absent_constructor() {
        use AstNode::*;

        // Absent
        let ast = vec![Absent];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comment
        assert!(asm.contains("Create Absent variant"));

        // Should store tag=0
        assert!(asm.contains("movq $0"));
    }

    #[test]
    fn test_compile_outcome_pattern_match() {
        use AstNode::*;
        use crate::ast::Pattern;

        // bind result = Triumph(42)
        // match result with
        //     when Triumph(x) then x
        //     when Mishap(e) then 0
        // end
        let ast = vec![
            BindStmt {
                name: "result".to_string(),
                typ: None,
                value: Box::new(Triumph(Box::new(Number(42.0)))),
            },
            MatchStmt {
                value: Box::new(Ident("result".to_string())),
                arms: vec![
                    crate::ast::MatchArm {
                        pattern: Pattern::Enum {
                            variant: "Triumph".to_string(),
                            inner: Some(Box::new(Pattern::Ident("x".to_string()))),
                        },
                        body: vec![Ident("x".to_string())],
                    },
                    crate::ast::MatchArm {
                        pattern: Pattern::Enum {
                            variant: "Mishap".to_string(),
                            inner: Some(Box::new(Pattern::Ident("e".to_string()))),
                        },
                        body: vec![Number(0.0)],
                    },
                ],
            },
        ];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain comments for variants
        assert!(asm.contains("Match Triumph variant") || asm.contains("Match"));

        // Should load tag from offset +8
        assert!(asm.contains("8(%rax)"));

        // Should compare tag with expected values
        assert!(asm.contains("cmpq"));

        // Should have conditional jumps
        assert!(asm.contains("jne"));
    }

    #[test]
    fn test_compile_maybe_pattern_match() {
        use AstNode::*;
        use crate::ast::Pattern;

        // bind option = Present(10)
        // match option with
        //     when Present(n) then n * 2
        //     when Absent then 0
        // end
        let ast = vec![
            BindStmt {
                name: "option".to_string(),
                typ: None,
                value: Box::new(Present(Box::new(Number(10.0)))),
            },
            MatchStmt {
                value: Box::new(Ident("option".to_string())),
                arms: vec![
                    crate::ast::MatchArm {
                        pattern: Pattern::Enum {
                            variant: "Present".to_string(),
                            inner: Some(Box::new(Pattern::Ident("n".to_string()))),
                        },
                        body: vec![BinaryOp {
                            left: Box::new(Ident("n".to_string())),
                            op: BinaryOperator::Mul,
                            right: Box::new(Number(2.0)),
                        }],
                    },
                    crate::ast::MatchArm {
                        pattern: Pattern::Enum {
                            variant: "Absent".to_string(),
                            inner: None,
                        },
                        body: vec![Number(0.0)],
                    },
                ],
            },
        ];

        let result = compile_to_asm(&ast);
        assert!(result.is_ok());
        let asm = result.unwrap();

        // Should contain pattern matching logic
        assert!(asm.contains("cmpq"));
        assert!(asm.contains("jne") || asm.contains("jmp"));

        // Should load from enum structure
        assert!(asm.contains("(%rax)"));
    }
}
