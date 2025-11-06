//! # Bytecode Compiler
//!
//! Compiles Glimmer-Weave AST to Quicksilver bytecode.
//!
//! ## Register Allocation
//!
//! Uses a simple stack-based register allocator:
//! - r0-r7: Reserved for common operations
//! - r8+: Temporary registers allocated on demand
//!
//! ## Compilation Strategy
//!
//! 1. Walk AST depth-first
//! 2. Allocate registers for intermediate values
//! 3. Generate type-aware instructions
//! 4. Optimize simple patterns (constant folding, etc.)

use crate::ast::{AstNode, BinaryOperator, UnaryOperator, Parameter};
use crate::bytecode::{BytecodeChunk, Constant, Instruction, Register, ConstantId};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Compilation error
#[derive(Debug, Clone)]
pub enum CompileError {
    /// Too many registers needed
    TooManyRegisters,
    /// Too many constants
    TooManyConstants,
    /// Undefined variable
    UndefinedVariable(String),
    /// Unsupported feature
    UnsupportedFeature(String),
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Variable location
#[derive(Debug, Clone)]
enum VarLocation {
    /// Local variable at stack index
    Local(u8),
    /// Global variable by name
    Global(String),
}

/// Compilation scope
struct Scope {
    /// Variables defined in this scope
    variables: BTreeMap<String, VarLocation>,
    /// Parent scope depth
    depth: usize,
}

impl Scope {
    fn new(depth: usize) -> Self {
        Scope {
            variables: BTreeMap::new(),
            depth,
        }
    }
}

/// Bytecode compiler
pub struct BytecodeCompiler {
    /// Current chunk being compiled
    chunk: BytecodeChunk,

    /// Register allocator state
    next_register: Register,

    /// Maximum register used (for debugging)
    max_register: Register,

    /// Scope stack
    scopes: Vec<Scope>,

    /// Current local variable count
    local_count: u8,

    /// Current function name (for TCO detection)
    current_function: Option<String>,

    /// Current function entry point (for TCO jumps)
    function_entry: Option<usize>,
}

impl BytecodeCompiler {
    /// Create a new bytecode compiler
    pub fn new(name: String) -> Self {
        BytecodeCompiler {
            chunk: BytecodeChunk::new(name),
            next_register: 0,
            max_register: 0,
            scopes: vec![Scope::new(0)], // Global scope
            local_count: 0,
            current_function: None,
            function_entry: None,
        }
    }

    /// Compile a list of statements
    pub fn compile(&mut self, nodes: &[AstNode]) -> CompileResult<BytecodeChunk> {
        let mut last_result: Option<Register> = None;

        // Compile all statements
        for node in nodes {
            last_result = self.compile_stmt(node)?;
        }

        // If the last statement produced a result, move it to r0
        if let Some(result_reg) = last_result {
            if result_reg != 0 {
                self.emit(Instruction::Move { dest: 0, src: result_reg }, 0);
            }
        }

        // Emit halt at the end (returns r0)
        self.emit(Instruction::Halt, 0);

        // Return the completed chunk
        Ok(self.chunk.clone())
    }

    /// Compile a statement (returns register containing result, or None)
    fn compile_stmt(&mut self, node: &AstNode) -> CompileResult<Option<Register>> {
        match node {
            AstNode::BindStmt { name, typ: _, value } => {
                // Compile the value expression
                let value_reg = self.compile_expr(value)?;

                // Define the variable
                if self.scopes.len() == 1 {
                    // Global scope
                    let name_id = self.add_string_constant(name.clone());
                    self.emit(Instruction::DefineGlobal { name_id, src: value_reg }, 0);
                    self.current_scope_mut().variables.insert(name.clone(), VarLocation::Global(name.clone()));
                } else {
                    // Local scope
                    let local_index = self.local_count;
                    self.local_count += 1;
                    self.chunk.local_count = self.local_count;
                    self.emit(Instruction::StoreLocal { local_index, src: value_reg }, 0);
                    self.current_scope_mut().variables.insert(name.clone(), VarLocation::Local(local_index));
                }

                self.free_register(value_reg);
                Ok(None)
            }

            AstNode::WeaveStmt { name, typ: _, value } => {
                // Same as bind for now (mutability handled at runtime)
                self.compile_stmt(&AstNode::BindStmt {
                    name: name.clone(),
                    typ: None,
                    value: value.clone(),
                })
            }

            AstNode::SetStmt { name, value } => {
                // Compile the value
                let value_reg = self.compile_expr(value)?;

                // Store to variable
                let location = self.resolve_variable(name)?;
                match location {
                    VarLocation::Local(index) => {
                        self.emit(Instruction::StoreLocal { local_index: index, src: value_reg }, 0);
                    }
                    VarLocation::Global(_) => {
                        let name_id = self.add_string_constant(name.clone());
                        self.emit(Instruction::StoreGlobal { name_id, src: value_reg }, 0);
                    }
                }

                self.free_register(value_reg);
                Ok(None)
            }

            AstNode::IfStmt { condition, then_branch, else_branch } => {
                // Compile condition
                let cond_reg = self.compile_expr(condition)?;

                // Jump to else if condition is false
                self.emit(Instruction::JumpIfFalse { cond: cond_reg, offset: 0 }, 0);
                let jump_to_else = self.chunk.offset() - 1;

                self.free_register(cond_reg);

                // Compile then branch
                for stmt in then_branch {
                    self.compile_stmt(stmt)?;
                }

                // Jump over else branch
                self.emit(Instruction::Jump { offset: 0 }, 0);
                let jump_over_else = self.chunk.offset() - 1;

                // Patch jump to else
                let else_offset = self.chunk.offset();
                self.chunk.patch_jump(jump_to_else, else_offset);

                // Compile else branch if present
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.compile_stmt(stmt)?;
                    }
                }

                // Patch jump over else
                let end_offset = self.chunk.offset();
                self.chunk.patch_jump(jump_over_else, end_offset);

                Ok(None)
            }

            AstNode::WhileStmt { condition, body } => {
                let loop_start = self.chunk.offset();

                // Compile condition
                let cond_reg = self.compile_expr(condition)?;

                // Jump to end if condition is false
                self.emit(Instruction::JumpIfFalse { cond: cond_reg, offset: 0 }, 0);
                let jump_to_end = self.chunk.offset() - 1;

                self.free_register(cond_reg);

                // Compile loop body
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }

                // Jump back to start
                let offset_to_start = (loop_start as isize - self.chunk.offset() as isize - 1) as i16;
                self.emit(Instruction::Jump { offset: offset_to_start }, 0);

                // Patch jump to end
                let end_offset = self.chunk.offset();
                self.chunk.patch_jump(jump_to_end, end_offset);

                Ok(None)
            }

            AstNode::MatchStmt { value, arms } => {
                use crate::ast::Pattern;

                // Compile the value to match against
                let match_value_reg = self.compile_expr(value)?;

                // Track jumps to end (for successful matches)
                let mut jumps_to_end = Vec::new();

                // Compile each arm
                for arm in arms {
                    // Push new scope for pattern variables
                    self.scopes.push(Scope::new(self.scopes.len()));
                    let scope_local_start = self.local_count;

                    // Compile pattern matching logic
                    match &arm.pattern {
                        Pattern::Literal(lit_node) => {
                            // Compile the literal value
                            let lit_reg = self.compile_expr(lit_node)?;

                            // Compare match_value == literal
                            let cmp_reg = self.alloc_register()?;
                            self.emit(Instruction::Eq {
                                dest: cmp_reg,
                                left: match_value_reg,
                                right: lit_reg
                            }, 0);
                            self.free_register(lit_reg);

                            // Jump to next arm if not equal
                            self.emit(Instruction::JumpIfFalse { cond: cmp_reg, offset: 0 }, 0);
                            let jump_to_next_arm = self.chunk.offset() - 1;
                            self.free_register(cmp_reg);

                            // Pattern matched! Execute arm body
                            let mut result_reg = None;
                            for stmt in &arm.body {
                                result_reg = self.compile_stmt(stmt)?;
                            }

                            // If arm produced a result, move it to a temp register
                            if let Some(reg) = result_reg {
                                // Keep result in register for now
                                jumps_to_end.push((self.chunk.offset(), reg));
                                self.emit(Instruction::Jump { offset: 0 }, 0);
                            } else {
                                jumps_to_end.push((self.chunk.offset(), match_value_reg));
                                self.emit(Instruction::Jump { offset: 0 }, 0);
                            }

                            // Patch jump to next arm
                            let next_arm_offset = self.chunk.offset();
                            self.chunk.patch_jump(jump_to_next_arm, next_arm_offset);
                        }

                        Pattern::Ident(var_name) => {
                            // Variable binding - always matches
                            // Store match_value to a local variable
                            let local_index = self.local_count;
                            self.local_count += 1;
                            self.chunk.local_count = self.local_count;
                            self.emit(Instruction::StoreLocal {
                                local_index,
                                src: match_value_reg
                            }, 0);
                            self.current_scope_mut().variables.insert(
                                var_name.clone(),
                                VarLocation::Local(local_index)
                            );

                            // Execute arm body
                            let mut result_reg = None;
                            for stmt in &arm.body {
                                result_reg = self.compile_stmt(stmt)?;
                            }

                            // Jump to end
                            if let Some(reg) = result_reg {
                                jumps_to_end.push((self.chunk.offset(), reg));
                            } else {
                                jumps_to_end.push((self.chunk.offset(), match_value_reg));
                            }
                            self.emit(Instruction::Jump { offset: 0 }, 0);
                        }

                        Pattern::Wildcard => {
                            // Wildcard - always matches, no binding
                            // Execute arm body
                            let mut result_reg = None;
                            for stmt in &arm.body {
                                result_reg = self.compile_stmt(stmt)?;
                            }

                            // Jump to end
                            if let Some(reg) = result_reg {
                                jumps_to_end.push((self.chunk.offset(), reg));
                            } else {
                                jumps_to_end.push((self.chunk.offset(), match_value_reg));
                            }
                            self.emit(Instruction::Jump { offset: 0 }, 0);
                        }

                        Pattern::Enum { variant, inner } => {
                            // Check variant tag
                            let check_reg = self.alloc_register()?;
                            let instruction = match variant.as_str() {
                                "Triumph" => Instruction::IsTriumph { dest: check_reg, value: match_value_reg },
                                "Mishap" => Instruction::IsMishap { dest: check_reg, value: match_value_reg },
                                "Present" => Instruction::IsPresent { dest: check_reg, value: match_value_reg },
                                "Absent" => Instruction::IsAbsent { dest: check_reg, value: match_value_reg },
                                _ => return Err(CompileError::UnsupportedFeature(
                                    format!("Unknown enum variant: {}", variant)
                                )),
                            };
                            self.emit(instruction, 0);

                            // Jump to next arm if variant doesn't match
                            self.emit(Instruction::JumpIfFalse { cond: check_reg, offset: 0 }, 0);
                            let jump_to_next_arm = self.chunk.offset() - 1;
                            self.free_register(check_reg);

                            // If we have an inner pattern, extract and match it
                            if let Some(inner_pattern) = inner {
                                // Extract inner value (if variant has one)
                                if variant != "Absent" {
                                    let inner_reg = self.alloc_register()?;
                                    self.emit(Instruction::ExtractInner {
                                        dest: inner_reg,
                                        value: match_value_reg,
                                    }, 0);

                                    // Match inner pattern
                                    match inner_pattern.as_ref() {
                                        Pattern::Ident(var_name) => {
                                            // Bind to local variable
                                            let local_index = self.local_count;
                                            self.local_count += 1;
                                            self.chunk.local_count = self.local_count;
                                            self.emit(Instruction::StoreLocal {
                                                local_index,
                                                src: inner_reg,
                                            }, 0);
                                            self.current_scope_mut().variables.insert(
                                                var_name.clone(),
                                                VarLocation::Local(local_index)
                                            );
                                        }
                                        Pattern::Wildcard => {
                                            // Wildcard - no binding needed
                                        }
                                        _ => {
                                            // For complex nested patterns, would need recursive implementation
                                            return Err(CompileError::UnsupportedFeature(
                                                "Complex nested enum patterns not yet supported".to_string()
                                            ));
                                        }
                                    }

                                    self.free_register(inner_reg);
                                }
                            }

                            // Pattern matched! Execute arm body
                            let mut result_reg = None;
                            for stmt in &arm.body {
                                result_reg = self.compile_stmt(stmt)?;
                            }

                            // Jump to end
                            if let Some(reg) = result_reg {
                                jumps_to_end.push((self.chunk.offset(), reg));
                            } else {
                                jumps_to_end.push((self.chunk.offset(), match_value_reg));
                            }
                            self.emit(Instruction::Jump { offset: 0 }, 0);

                            // Patch jump to next arm
                            let next_arm_offset = self.chunk.offset();
                            self.chunk.patch_jump(jump_to_next_arm, next_arm_offset);
                        }
                    }

                    // Pop scope and restore local count
                    self.scopes.pop();
                    self.local_count = scope_local_start;
                }

                // Patch all jumps to end
                let end_offset = self.chunk.offset();
                for (jump_offset, _result_reg) in &jumps_to_end {
                    self.chunk.patch_jump(*jump_offset, end_offset);
                }

                self.free_register(match_value_reg);

                // Match statement doesn't produce a result by default
                Ok(None)
            }

            AstNode::ChantDef { name, params, return_type: _, body } => {
                // For now, create a simple inline function
                // Store function entry point for TCO
                let old_function = self.current_function.clone();
                let old_entry = self.function_entry;

                self.current_function = Some(name.clone());
                self.function_entry = Some(self.chunk.offset());

                // Push new scope for function
                self.scopes.push(Scope::new(self.scopes.len()));

                // Bind parameters as locals
                for param in params {
                    let local_index = self.local_count;
                    self.local_count += 1;
                    self.chunk.local_count = self.local_count;
                    self.current_scope_mut().variables.insert(
                        param.name.clone(),
                        VarLocation::Local(local_index)
                    );
                }

                // Compile function body
                let mut last_reg = None;
                for stmt in body {
                    last_reg = self.compile_stmt(stmt)?;
                }

                // If no explicit yield, return nothing
                if last_reg.is_none() {
                    let reg = self.alloc_register()?;
                    self.emit(Instruction::LoadNothing { dest: reg }, 0);
                    self.emit(Instruction::Return { value: reg }, 0);
                    self.free_register(reg);
                }

                // Restore previous function context
                self.scopes.pop();
                self.current_function = old_function;
                self.function_entry = old_entry;

                Ok(None)
            }

            AstNode::FormDef { name, fields } => {
                // Create struct definition as a constant
                let struct_def_id = self.chunk.add_constant(Constant::StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                });

                // Store struct definition as a global variable
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::LoadConst {
                    dest: dest_reg,
                    constant_id: struct_def_id,
                }, 0);

                // Add name as a constant for StoreGlobal
                let name_id = self.chunk.add_constant(Constant::Text(name.clone()));
                self.emit(Instruction::StoreGlobal {
                    name_id,
                    src: dest_reg,
                }, 0);

                self.free_register(dest_reg);

                Ok(None)
            }

            AstNode::YieldStmt { value } => {
                // Check for tail call (yield f(args) where f is current function)
                if let AstNode::Call { callee, args } = value.as_ref() {
                    if let AstNode::Ident(func_name) = callee.as_ref() {
                        if Some(func_name) == self.current_function.as_ref() {
                            // This is a tail call! Use TCO.
                            // Evaluate arguments
                            let mut arg_regs = Vec::new();
                            for arg in args {
                                let reg = self.compile_expr(arg)?;
                                arg_regs.push(reg);
                            }

                            // Update parameter locals with new values
                            for (i, arg_reg) in arg_regs.iter().enumerate() {
                                self.emit(Instruction::StoreLocal {
                                    local_index: i as u8,
                                    src: *arg_reg
                                }, 0);
                                self.free_register(*arg_reg);
                            }

                            // Jump back to function start (TCO!)
                            if let Some(entry) = self.function_entry {
                                let current_offset = self.chunk.offset();
                                let jump_offset = (entry as isize - current_offset as isize - 1) as i16;
                                self.emit(Instruction::Jump { offset: jump_offset }, 0);
                            }

                            return Ok(None);
                        }
                    }
                }

                // Not a tail call, emit normal return
                let reg = self.compile_expr(value)?;
                self.emit(Instruction::Return { value: reg }, 0);
                self.free_register(reg);
                Ok(None)
            }

            AstNode::AttemptStmt { body, handlers } => {
                // Setup exception handler
                // Emit SetupTry with placeholder offset (will be patched)
                self.emit(Instruction::SetupTry { handler_offset: 0 }, 0);
                let setup_try_index = self.chunk.offset() - 1;

                // Compile the try body
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }

                // If we get here, no error occurred
                // Pop the exception handler
                self.emit(Instruction::PopTry, 0);

                // Jump over handler code
                self.emit(Instruction::Jump { offset: 0 }, 0);
                let jump_over_handlers = self.chunk.offset() - 1;

                // Patch SetupTry to point to handler code
                let handler_start = self.chunk.offset();
                self.chunk.patch_jump(setup_try_index, handler_start);

                // The error type and value will be in registers set by the VM
                // r254: error type (as Text)
                // r255: error value (as Text)

                // Compile each error handler
                for (i, handler) in handlers.iter().enumerate() {
                    // Check if error type matches
                    let error_type_reg = 254; // VM sets this
                    let expected_type_reg = self.alloc_register()?;

                    // Load expected error type as constant
                    let type_const_id = self.chunk.add_constant(
                        Constant::Text(handler.error_type.clone())
                    );
                    self.emit(Instruction::LoadConst {
                        dest: expected_type_reg,
                        constant_id: type_const_id
                    }, 0);

                    // Compare error type (or check for wildcard "_")
                    let matches_reg = self.alloc_register()?;
                    if handler.error_type == "_" {
                        // Wildcard - always matches
                        self.emit(Instruction::LoadTruth {
                            dest: matches_reg,
                            value: true
                        }, 0);
                    } else {
                        // Check if types match
                        self.emit(Instruction::Eq {
                            dest: matches_reg,
                            left: error_type_reg,
                            right: expected_type_reg
                        }, 0);
                    }

                    self.free_register(expected_type_reg);

                    // Jump to next handler if not matched
                    self.emit(Instruction::JumpIfFalse {
                        cond: matches_reg,
                        offset: 0
                    }, 0);
                    let jump_to_next_handler = self.chunk.offset() - 1;
                    self.free_register(matches_reg);

                    // This handler matches! Execute its body
                    for stmt in &handler.body {
                        self.compile_stmt(stmt)?;
                    }

                    // Pop the exception handler (error was handled)
                    self.emit(Instruction::PopTry, 0);

                    // Jump to end
                    self.emit(Instruction::Jump { offset: 0 }, 0);
                    let jump_to_end = self.chunk.offset() - 1;

                    // Patch jump to next handler
                    let next_handler_offset = self.chunk.offset();
                    self.chunk.patch_jump(jump_to_next_handler, next_handler_offset);

                    // Patch jump to end to point past all handlers
                    // We'll patch this at the very end
                    if i == handlers.len() - 1 {
                        // This is the last handler
                        // If we get here, no handler matched - re-throw error
                        let error_reg = 255; // VM sets this
                        self.emit(Instruction::Throw { error_reg }, 0);
                    }

                    // Patch the jump to end
                    let end_offset = self.chunk.offset();
                    self.chunk.patch_jump(jump_to_end, end_offset);
                }

                // Patch jump over handlers
                let final_offset = self.chunk.offset();
                self.chunk.patch_jump(jump_over_handlers, final_offset);

                Ok(None)
            }

            AstNode::ExprStmt(expr) => {
                let reg = self.compile_expr(expr)?;
                // Don't free the register - return it as the result
                Ok(Some(reg))
            }

            _ => {
                // Try compiling as expression
                let reg = self.compile_expr(node)?;
                // Don't free the register - we're returning it as the result
                Ok(Some(reg))
            }
        }
    }

    /// Compile an expression (returns register containing result)
    fn compile_expr(&mut self, node: &AstNode) -> CompileResult<Register> {
        match node {
            AstNode::Number(n) => {
                let reg = self.alloc_register()?;
                let const_id = self.chunk.add_constant(Constant::Number(*n));
                self.emit(Instruction::LoadConst { dest: reg, constant_id: const_id }, 0);
                Ok(reg)
            }

            AstNode::Text(s) => {
                let reg = self.alloc_register()?;
                let const_id = self.chunk.add_constant(Constant::Text(s.clone()));
                self.emit(Instruction::LoadConst { dest: reg, constant_id: const_id }, 0);
                Ok(reg)
            }

            AstNode::Truth(b) => {
                let reg = self.alloc_register()?;
                self.emit(Instruction::LoadTruth { dest: reg, value: *b }, 0);
                Ok(reg)
            }

            AstNode::Nothing => {
                let reg = self.alloc_register()?;
                self.emit(Instruction::LoadNothing { dest: reg }, 0);
                Ok(reg)
            }

            AstNode::Ident(name) => {
                let reg = self.alloc_register()?;
                let location = self.resolve_variable(name)?;

                match location {
                    VarLocation::Local(index) => {
                        self.emit(Instruction::LoadLocal { dest: reg, local_index: index }, 0);
                    }
                    VarLocation::Global(_) => {
                        let name_id = self.add_string_constant(name.clone());
                        self.emit(Instruction::LoadGlobal { dest: reg, name_id }, 0);
                    }
                }

                Ok(reg)
            }

            AstNode::BinaryOp { left, op, right } => {
                self.compile_binary_op(left, *op, right)
            }

            AstNode::UnaryOp { op, operand } => {
                self.compile_unary_op(*op, operand)
            }

            AstNode::List(elements) => {
                // Compile all elements into consecutive registers
                let start_reg = self.next_register;
                let mut regs = Vec::new();

                for elem in elements {
                    let reg = self.compile_expr(elem)?;
                    regs.push(reg);
                }

                // Create list from registers
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateList {
                    dest: dest_reg,
                    start: start_reg,
                    count: regs.len() as u8,
                }, 0);

                // Free element registers
                for reg in regs {
                    self.free_register(reg);
                }

                Ok(dest_reg)
            }

            AstNode::Map(fields) => {
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateMap { dest: dest_reg }, 0);

                // Set each field
                for (field_name, value_node) in fields {
                    let value_reg = self.compile_expr(value_node)?;
                    let field_id = self.add_string_constant(field_name.clone());
                    self.emit(Instruction::SetField {
                        map: dest_reg,
                        field_id,
                        value: value_reg,
                    }, 0);
                    self.free_register(value_reg);
                }

                Ok(dest_reg)
            }

            AstNode::IndexAccess { object, index } => {
                let list_reg = self.compile_expr(object)?;
                let index_reg = self.compile_expr(index)?;
                let dest_reg = self.alloc_register()?;

                self.emit(Instruction::GetIndex {
                    dest: dest_reg,
                    list: list_reg,
                    index: index_reg,
                }, 0);

                self.free_register(list_reg);
                self.free_register(index_reg);

                Ok(dest_reg)
            }

            AstNode::FieldAccess { object, field } => {
                let map_reg = self.compile_expr(object)?;
                let field_id = self.add_string_constant(field.clone());
                let dest_reg = self.alloc_register()?;

                self.emit(Instruction::GetField {
                    dest: dest_reg,
                    map: map_reg,
                    field_id,
                }, 0);

                self.free_register(map_reg);

                Ok(dest_reg)
            }

            AstNode::Call { callee, args } => {
                // Compile callee (should be a function value)
                let func_reg = self.compile_expr(callee)?;

                // Compile arguments into consecutive registers
                let arg_start = self.next_register;
                let mut arg_regs = Vec::new();
                for arg in args {
                    let reg = self.compile_expr(arg)?;
                    arg_regs.push(reg);
                }

                // Emit call instruction
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::Call {
                    dest: dest_reg,
                    func: func_reg,
                    arg_start,
                    arg_count: arg_regs.len() as u8,
                }, 0);

                // Free argument registers
                for reg in arg_regs {
                    self.free_register(reg);
                }
                self.free_register(func_reg);

                Ok(dest_reg)
            }

            // Enum constructors
            AstNode::Triumph(value) => {
                let value_reg = self.compile_expr(value)?;
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateTriumph {
                    dest: dest_reg,
                    value: value_reg,
                }, 0);
                self.free_register(value_reg);
                Ok(dest_reg)
            }

            AstNode::Mishap(value) => {
                let value_reg = self.compile_expr(value)?;
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateMishap {
                    dest: dest_reg,
                    value: value_reg,
                }, 0);
                self.free_register(value_reg);
                Ok(dest_reg)
            }

            AstNode::Present(value) => {
                let value_reg = self.compile_expr(value)?;
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreatePresent {
                    dest: dest_reg,
                    value: value_reg,
                }, 0);
                self.free_register(value_reg);
                Ok(dest_reg)
            }

            AstNode::Absent => {
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateAbsent {
                    dest: dest_reg,
                }, 0);
                Ok(dest_reg)
            }

            AstNode::StructLiteral { struct_name, fields } => {
                // Look up the struct definition (it should be a global)
                // For now, we'll use the struct name as a constant ID reference
                let struct_def_id = self.chunk.add_constant(Constant::Text(struct_name.clone()));

                // Compile field values and allocate consecutive registers
                let field_start = self.alloc_register()?;
                let mut field_regs = vec![field_start];

                // Evaluate field values in order of field names
                // Note: Fields should be in the same order as the struct definition
                for (_field_name, field_value) in fields {
                    let field_reg = self.compile_expr(field_value)?;
                    field_regs.push(field_reg);
                }

                // Create the struct instance
                let dest_reg = self.alloc_register()?;
                self.emit(Instruction::CreateStruct {
                    dest: dest_reg,
                    struct_def_id,
                    field_start,
                    field_count: fields.len() as u8,
                }, 0);

                // Free field registers
                for reg in field_regs {
                    self.free_register(reg);
                }

                Ok(dest_reg)
            }

            _ => Err(CompileError::UnsupportedFeature(format!("{:?}", node))),
        }
    }

    /// Compile a binary operation
    fn compile_binary_op(&mut self, left: &AstNode, op: BinaryOperator, right: &AstNode) -> CompileResult<Register> {
        let left_reg = self.compile_expr(left)?;
        let right_reg = self.compile_expr(right)?;
        let dest_reg = self.alloc_register()?;

        let instruction = match op {
            BinaryOperator::Add => {
                // TODO: Type-aware dispatch (AddNum vs ConcatText)
                // For now, emit AddNum (runtime will handle type checking)
                Instruction::AddNum { dest: dest_reg, left: left_reg, right: right_reg }
            }
            BinaryOperator::Sub => Instruction::SubNum { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Mul => Instruction::MulNum { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Div => Instruction::DivNum { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Mod => Instruction::ModNum { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Equal => Instruction::Eq { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::NotEqual => Instruction::Ne { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Greater => Instruction::Gt { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Less => Instruction::Lt { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::GreaterEq => Instruction::Ge { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::LessEq => Instruction::Le { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::And => Instruction::And { dest: dest_reg, left: left_reg, right: right_reg },
            BinaryOperator::Or => Instruction::Or { dest: dest_reg, left: left_reg, right: right_reg },
        };

        self.emit(instruction, 0);
        self.free_register(left_reg);
        self.free_register(right_reg);

        Ok(dest_reg)
    }

    /// Compile a unary operation
    fn compile_unary_op(&mut self, op: UnaryOperator, operand: &AstNode) -> CompileResult<Register> {
        let operand_reg = self.compile_expr(operand)?;
        let dest_reg = self.alloc_register()?;

        let instruction = match op {
            UnaryOperator::Not => Instruction::Not { dest: dest_reg, src: operand_reg },
            UnaryOperator::Negate => Instruction::NegNum { dest: dest_reg, src: operand_reg },
        };

        self.emit(instruction, 0);
        self.free_register(operand_reg);

        Ok(dest_reg)
    }

    /// Allocate a register
    fn alloc_register(&mut self) -> CompileResult<Register> {
        if self.next_register >= 255 {
            return Err(CompileError::TooManyRegisters);
        }

        let reg = self.next_register;
        self.next_register += 1;

        if reg > self.max_register {
            self.max_register = reg;
        }

        Ok(reg)
    }

    /// Free a register (simple stack-based allocator)
    fn free_register(&mut self, _reg: Register) {
        // In a stack-based allocator, we pop the most recent register
        if self.next_register > 0 {
            self.next_register -= 1;
        }
    }

    /// Emit an instruction
    fn emit(&mut self, instruction: Instruction, line: usize) {
        self.chunk.emit(instruction, line);
    }

    /// Add a string constant to the pool
    fn add_string_constant(&mut self, s: String) -> ConstantId {
        self.chunk.add_constant(Constant::Text(s))
    }

    /// Get current scope
    fn current_scope(&self) -> &Scope {
        self.scopes.last().expect("No scope available")
    }

    /// Get current scope mutably
    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("No scope available")
    }

    /// Resolve a variable to its location
    fn resolve_variable(&self, name: &str) -> CompileResult<VarLocation> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(location) = scope.variables.get(name) {
                return Ok(location.clone());
            }
        }

        Err(CompileError::UndefinedVariable(name.to_string()))
    }
}

/// Compile Glimmer-Weave AST to bytecode
pub fn compile(nodes: &[AstNode]) -> CompileResult<BytecodeChunk> {
    let mut compiler = BytecodeCompiler::new("main".to_string());
    compiler.compile(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn compile_source(source: &str) -> CompileResult<BytecodeChunk> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse failed");
        compile(&ast)
    }

    #[test]
    fn test_compile_simple_number() {
        let chunk = compile_source("42").expect("Compile failed");
        assert!(chunk.instructions.len() > 0);
        assert_eq!(chunk.constants.len(), 1);
        assert_eq!(chunk.constants[0], Constant::Number(42.0));
    }

    #[test]
    fn test_compile_arithmetic() {
        let chunk = compile_source("10 + 20").expect("Compile failed");
        // Should have: LOAD_CONST, LOAD_CONST, ADD_NUM, HALT
        assert!(chunk.instructions.len() >= 3);
    }

    #[test]
    fn test_compile_bind() {
        let chunk = compile_source("bind x to 42").expect("Compile failed");
        // Should have: LOAD_CONST, DEF_GLOBAL, HALT
        assert!(chunk.instructions.len() >= 2);
    }

    #[test]
    fn test_compile_if_statement() {
        let chunk = compile_source(r#"
            should true then
                42
            end
        "#).expect("Compile failed");
        // Should have jumps and conditional logic
        assert!(chunk.instructions.len() >= 4);
    }

    #[test]
    fn test_compile_while_loop() {
        let chunk = compile_source(r#"
            weave i as 0
            whilst i less than 10 then
                set i to i + 1
            end
        "#).expect("Compile failed");
        // Should have jumps for loop
        assert!(chunk.instructions.len() >= 5);
    }

    #[test]
    fn test_compile_tco_recursion() {
        let chunk = compile_source(r#"
            chant sum_to(n, acc) then
                should n at most 0 then
                    yield acc
                otherwise
                    yield sum_to(n - 1, acc + n)
                end
            end
        "#).expect("Compile failed");

        // Should have Jump instruction for TCO (not Call+Return)
        // Find the tail-recursive jump
        let has_jump_back = chunk.instructions.iter().any(|inst| {
            matches!(inst, Instruction::Jump { offset } if *offset < 0)
        });
        assert!(has_jump_back, "TCO should emit a backwards jump");
    }

    #[test]
    fn test_compile_pattern_matching() {
        let chunk = compile_source(r#"
            bind x to 2

            match x with
                when 1 then "one"
                when 2 then "two"
                when 3 then "three"
                otherwise then "other"
            end
        "#).expect("Compile failed");

        // Should have comparison and conditional jump instructions
        let has_eq = chunk.instructions.iter().any(|inst| {
            matches!(inst, Instruction::Eq { .. })
        });
        let has_jump_if_false = chunk.instructions.iter().any(|inst| {
            matches!(inst, Instruction::JumpIfFalse { .. })
        });

        assert!(has_eq, "Pattern matching should emit Eq instruction");
        assert!(has_jump_if_false, "Pattern matching should emit JumpIfFalse");
    }

    #[test]
    fn test_compile_pattern_matching_with_binding() {
        let chunk = compile_source(r#"
            bind x to 42

            match x with
                when 0 then "zero"
                when n then n * 2
            end
        "#).expect("Compile failed");

        // Should have StoreLocal for variable binding
        let has_store_local = chunk.instructions.iter().any(|inst| {
            matches!(inst, Instruction::StoreLocal { .. })
        });

        assert!(has_store_local, "Pattern binding should emit StoreLocal");
    }
}
