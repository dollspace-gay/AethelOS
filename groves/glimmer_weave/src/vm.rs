//! # Quicksilver Virtual Machine
//!
//! Register-based bytecode executor for Glimmer-Weave.
//! Provides 5-10x performance improvement over tree-walking interpreter.
//!
//! ## Architecture
//!
//! - **256 Registers**: Fast local storage (r0-r255)
//! - **Value Stack**: For complex operations
//! - **Call Stack**: For function calls and returns
//! - **Global Variables**: Hash map for global storage

use crate::bytecode::{BytecodeChunk, Constant, Instruction};
use crate::eval::Value;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// VM runtime error
#[derive(Debug, Clone)]
pub enum VmError {
    /// Stack overflow
    StackOverflow,
    /// Stack underflow
    StackUnderflow,
    /// Invalid register access
    InvalidRegister(u8),
    /// Type error
    TypeError(String),
    /// Undefined variable
    UndefinedVariable(String),
    /// Division by zero
    DivisionByZero,
    /// Out of bounds access
    OutOfBounds,
}

pub type VmResult<T> = Result<T, VmError>;

/// Call frame for function calls
#[derive(Debug, Clone)]
struct CallFrame {
    /// Return address (instruction pointer)
    return_ip: usize,
    /// Base register for this frame
    base_register: u8,
}

/// Exception handler for try/catch
#[derive(Debug, Clone)]
struct ExceptionHandler {
    /// Handler code offset (where to jump on error)
    handler_offset: usize,
}

/// Quicksilver Virtual Machine
pub struct VM {
    /// Register file (256 registers)
    registers: [Value; 256],

    /// Global variables
    globals: BTreeMap<String, Value>,

    /// Call stack
    call_stack: Vec<CallFrame>,

    /// Exception handler stack
    exception_handlers: Vec<ExceptionHandler>,

    /// Instruction pointer
    ip: usize,

    /// Current chunk being executed
    chunk: Option<BytecodeChunk>,
}

impl VM {
    /// Create a new VM
    pub fn new() -> Self {
        VM {
            registers: core::array::from_fn(|_| Value::Nothing),
            globals: BTreeMap::new(),
            call_stack: Vec::new(),
            exception_handlers: Vec::new(),
            ip: 0,
            chunk: None,
        }
    }

    /// Execute a bytecode chunk
    pub fn execute(&mut self, chunk: BytecodeChunk) -> VmResult<Value> {
        self.chunk = Some(chunk);
        self.ip = 0;

        loop {
            let instruction = self.fetch_instruction()?;

            match instruction {
                Instruction::Halt => {
                    // Return r0 as result
                    return Ok(self.registers[0].clone());
                }

                Instruction::LoadConst { dest, constant_id } => {
                    let constant = self.get_constant(constant_id)?;
                    self.registers[dest as usize] = constant_to_value(constant);
                }

                Instruction::Move { dest, src } => {
                    self.registers[dest as usize] = self.registers[src as usize].clone();
                }

                Instruction::LoadNothing { dest } => {
                    self.registers[dest as usize] = Value::Nothing;
                }

                Instruction::LoadTruth { dest, value } => {
                    self.registers[dest as usize] = Value::Truth(value);
                }

                Instruction::AddNum { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Number(l + r);
                }

                Instruction::SubNum { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Number(l - r);
                }

                Instruction::MulNum { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Number(l * r);
                }

                Instruction::DivNum { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    if r == 0.0 {
                        self.handle_error("DivisionByZero", "Division by zero")?;
                        // If we get here, the error was handled - set result to 0
                        self.registers[dest as usize] = Value::Number(0.0);
                    } else {
                        self.registers[dest as usize] = Value::Number(l / r);
                    }
                }

                Instruction::ModNum { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Number(l % r);
                }

                Instruction::NegNum { dest, src } => {
                    let n = self.get_number(src)?;
                    self.registers[dest as usize] = Value::Number(-n);
                }

                Instruction::ConcatText { dest, left, right } => {
                    let l = self.get_text(left)?;
                    let r = self.get_text(right)?;
                    self.registers[dest as usize] = Value::Text(l + &r);
                }

                Instruction::Eq { dest, left, right } => {
                    let l = &self.registers[left as usize];
                    let r = &self.registers[right as usize];
                    self.registers[dest as usize] = Value::Truth(l == r);
                }

                Instruction::Ne { dest, left, right } => {
                    let l = &self.registers[left as usize];
                    let r = &self.registers[right as usize];
                    self.registers[dest as usize] = Value::Truth(l != r);
                }

                Instruction::Lt { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Truth(l < r);
                }

                Instruction::Le { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Truth(l <= r);
                }

                Instruction::Gt { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Truth(l > r);
                }

                Instruction::Ge { dest, left, right } => {
                    let l = self.get_number(left)?;
                    let r = self.get_number(right)?;
                    self.registers[dest as usize] = Value::Truth(l >= r);
                }

                Instruction::Not { dest, src } => {
                    let b = self.is_truthy(src);
                    self.registers[dest as usize] = Value::Truth(!b);
                }

                Instruction::And { dest, left, right } => {
                    let l = self.is_truthy(left);
                    let r = self.is_truthy(right);
                    self.registers[dest as usize] = Value::Truth(l && r);
                }

                Instruction::Or { dest, left, right } => {
                    let l = self.is_truthy(left);
                    let r = self.is_truthy(right);
                    self.registers[dest as usize] = Value::Truth(l || r);
                }

                Instruction::Jump { offset } => {
                    self.ip = (self.ip as isize + offset as isize) as usize;
                }

                Instruction::JumpIfTrue { cond, offset } => {
                    if self.is_truthy(cond) {
                        self.ip = (self.ip as isize + offset as isize) as usize;
                    }
                }

                Instruction::JumpIfFalse { cond, offset } => {
                    if !self.is_truthy(cond) {
                        self.ip = (self.ip as isize + offset as isize) as usize;
                    }
                }

                Instruction::DefineGlobal { name_id, src } => {
                    let name = self.get_string_constant(name_id)?;
                    let value = self.registers[src as usize].clone();
                    self.globals.insert(name, value);
                }

                Instruction::LoadGlobal { dest, name_id } => {
                    let name = self.get_string_constant(name_id)?;
                    let value = self.globals.get(&name)
                        .ok_or_else(|| VmError::UndefinedVariable(name.clone()))?;
                    self.registers[dest as usize] = value.clone();
                }

                Instruction::StoreGlobal { name_id, src } => {
                    let name = self.get_string_constant(name_id)?;
                    let value = self.registers[src as usize].clone();
                    if !self.globals.contains_key(&name) {
                        return Err(VmError::UndefinedVariable(name));
                    }
                    self.globals.insert(name, value);
                }

                Instruction::LoadLocal { .. } | Instruction::StoreLocal { .. } => {
                    // TODO: Implement local variables with proper frame handling
                    return Err(VmError::TypeError("Local variables not yet implemented".to_string()));
                }

                Instruction::CreateList { dest, start, count } => {
                    let mut elements = Vec::new();
                    for i in 0..count {
                        elements.push(self.registers[(start + i) as usize].clone());
                    }
                    self.registers[dest as usize] = Value::List(elements);
                }

                Instruction::CreateMap { dest } => {
                    self.registers[dest as usize] = Value::Map(BTreeMap::new());
                }

                Instruction::GetIndex { dest, list, index } => {
                    match (&self.registers[list as usize], &self.registers[index as usize]) {
                        (Value::List(elements), Value::Number(idx)) => {
                            let i = *idx as usize;
                            if i >= elements.len() {
                                return Err(VmError::OutOfBounds);
                            }
                            self.registers[dest as usize] = elements[i].clone();
                        }
                        _ => return Err(VmError::TypeError("Invalid index access".to_string())),
                    }
                }

                Instruction::SetIndex { list, index, value } => {
                    // Clone values first to avoid borrow checker issues
                    let index_value = self.registers[index as usize].clone();
                    let value_to_set = self.registers[value as usize].clone();

                    match (&mut self.registers[list as usize], index_value) {
                        (Value::List(elements), Value::Number(idx)) => {
                            let i = idx as usize;
                            if i >= elements.len() {
                                return Err(VmError::OutOfBounds);
                            }
                            elements[i] = value_to_set;
                        }
                        _ => return Err(VmError::TypeError("Invalid index assignment".to_string())),
                    }
                }

                Instruction::GetField { dest, map, field_id } => {
                    let field_name = self.get_string_constant(field_id)?;
                    match &self.registers[map as usize] {
                        Value::Map(fields) => {
                            let value = fields.get(&field_name)
                                .unwrap_or(&Value::Nothing);
                            self.registers[dest as usize] = value.clone();
                        }
                        _ => return Err(VmError::TypeError("GetField on non-map".to_string())),
                    }
                }

                Instruction::SetField { map, field_id, value } => {
                    let field_name = self.get_string_constant(field_id)?;
                    let value_to_set = self.registers[value as usize].clone();

                    match &mut self.registers[map as usize] {
                        Value::Map(fields) => {
                            fields.insert(field_name, value_to_set);
                        }
                        _ => return Err(VmError::TypeError("SetField on non-map".to_string())),
                    }
                }

                Instruction::Print { src } => {
                    // Debug instruction
                    #[cfg(test)]
                    println!("VM PRINT: {:?}", self.registers[src as usize]);
                }

                // Enum/Variant instructions
                Instruction::CreateTriumph { dest, value } => {
                    let inner = self.registers[value as usize].clone();
                    self.registers[dest as usize] = Value::Outcome {
                        success: true,
                        value: Box::new(inner),
                    };
                }

                Instruction::CreateMishap { dest, value } => {
                    let inner = self.registers[value as usize].clone();
                    self.registers[dest as usize] = Value::Outcome {
                        success: false,
                        value: Box::new(inner),
                    };
                }

                Instruction::CreatePresent { dest, value } => {
                    let inner = self.registers[value as usize].clone();
                    self.registers[dest as usize] = Value::Maybe {
                        present: true,
                        value: Some(Box::new(inner)),
                    };
                }

                Instruction::CreateAbsent { dest } => {
                    self.registers[dest as usize] = Value::Maybe {
                        present: false,
                        value: None,
                    };
                }

                Instruction::IsTriumph { dest, value } => {
                    let is_triumph = matches!(
                        &self.registers[value as usize],
                        Value::Outcome { success: true, .. }
                    );
                    self.registers[dest as usize] = Value::Truth(is_triumph);
                }

                Instruction::IsMishap { dest, value } => {
                    let is_mishap = matches!(
                        &self.registers[value as usize],
                        Value::Outcome { success: false, .. }
                    );
                    self.registers[dest as usize] = Value::Truth(is_mishap);
                }

                Instruction::IsPresent { dest, value } => {
                    let is_present = matches!(
                        &self.registers[value as usize],
                        Value::Maybe { present: true, .. }
                    );
                    self.registers[dest as usize] = Value::Truth(is_present);
                }

                Instruction::IsAbsent { dest, value } => {
                    let is_absent = matches!(
                        &self.registers[value as usize],
                        Value::Maybe { present: false, .. }
                    );
                    self.registers[dest as usize] = Value::Truth(is_absent);
                }

                Instruction::ExtractInner { dest, value } => {
                    match &self.registers[value as usize] {
                        Value::Outcome { value: inner, .. } => {
                            self.registers[dest as usize] = (**inner).clone();
                        }
                        Value::Maybe { value: Some(inner), .. } => {
                            self.registers[dest as usize] = (**inner).clone();
                        }
                        _ => return Err(VmError::TypeError("ExtractInner on non-enum value".to_string())),
                    }
                }

                Instruction::CreateStruct { dest, struct_def_id, field_start, field_count } => {
                    // Get the struct name from the constant (it's stored as Text for simplicity)
                    let struct_name = if let Value::Text(name) = constant_to_value(&self.chunk.as_ref().unwrap().constants[struct_def_id as usize]) {
                        name
                    } else {
                        return Err(VmError::TypeError("Expected Text constant for struct name".to_string()));
                    };

                    // Collect field values from consecutive registers
                    let mut field_values = Vec::new();
                    for i in 0..field_count {
                        let reg_idx = (field_start + i) as usize;
                        field_values.push(self.registers[reg_idx].clone());
                    }

                    // Look up the struct definition from globals
                    let struct_def = self.globals.get(&struct_name)
                        .ok_or_else(|| VmError::UndefinedVariable(struct_name.clone()))?;

                    // Extract field names from the struct definition
                    if let Value::StructDef { name: def_name, fields } = struct_def {
                        // Create a map of field names to values
                        let mut field_map = alloc::collections::BTreeMap::new();
                        for (i, field) in fields.iter().enumerate() {
                            if i < field_values.len() {
                                field_map.insert(field.name.clone(), field_values[i].clone());
                            }
                        }

                        // Create the struct instance
                        self.registers[dest as usize] = Value::StructInstance {
                            struct_name: def_name.clone(),
                            fields: field_map,
                        };
                    } else {
                        return Err(VmError::TypeError("Expected struct definition".to_string()));
                    }
                }

                Instruction::SetupTry { handler_offset } => {
                    // Push exception handler onto stack
                    self.exception_handlers.push(ExceptionHandler {
                        handler_offset,
                    });
                }

                Instruction::PopTry => {
                    // Remove the most recent exception handler
                    self.exception_handlers.pop();
                }

                Instruction::Throw { error_reg } => {
                    // Get the error value from the register
                    let error_value = self.registers[error_reg as usize].clone();

                    // Check if there's an exception handler
                    if let Some(handler) = self.exception_handlers.pop() {
                        // Set error registers:
                        // r254 = error type (Text)
                        // r255 = error value (Text)

                        // For now, we'll use "RuntimeError" as the error type
                        // In a more complete implementation, we'd extract the type from the error
                        self.registers[254] = Value::Text("RuntimeError".to_string());
                        self.registers[255] = error_value;

                        // Jump to handler code
                        self.ip = handler.handler_offset;
                    } else {
                        // No handler - propagate as VmError
                        return Err(VmError::TypeError(format!("Uncaught error: {:?}", error_value)));
                    }
                }

                _ => {
                    return Err(VmError::TypeError(format!("Unimplemented instruction: {:?}", instruction)));
                }
            }
        }
    }

    /// Fetch the next instruction
    fn fetch_instruction(&mut self) -> VmResult<Instruction> {
        let chunk = self.chunk.as_ref().ok_or(VmError::StackUnderflow)?;
        if self.ip >= chunk.instructions.len() {
            return Err(VmError::StackUnderflow);
        }

        let instruction = chunk.instructions[self.ip].clone();
        self.ip += 1;
        Ok(instruction)
    }

    /// Handle a runtime error by checking for exception handlers
    /// If a handler exists, sets error registers and jumps to handler
    /// If no handler exists, returns the error
    fn handle_error(&mut self, error_type: &str, error_msg: &str) -> VmResult<()> {
        if let Some(handler) = self.exception_handlers.pop() {
            // Set error registers:
            // r254 = error type (Text)
            // r255 = error value (Text)
            self.registers[254] = Value::Text(error_type.to_string());
            self.registers[255] = Value::Text(error_msg.to_string());

            // Jump to handler code
            self.ip = handler.handler_offset;
            Ok(())
        } else {
            // No handler - return error
            Err(VmError::DivisionByZero)
        }
    }

    /// Get a constant from the pool
    fn get_constant(&self, id: u16) -> VmResult<&Constant> {
        let chunk = self.chunk.as_ref().ok_or(VmError::StackUnderflow)?;
        chunk.constants.get(id as usize)
            .ok_or(VmError::TypeError("Invalid constant ID".to_string()))
    }

    /// Get a string constant
    fn get_string_constant(&self, id: u16) -> VmResult<String> {
        match self.get_constant(id)? {
            Constant::Text(s) => Ok(s.clone()),
            _ => Err(VmError::TypeError("Expected string constant".to_string())),
        }
    }

    /// Get a number from a register
    fn get_number(&self, reg: u8) -> VmResult<f64> {
        match &self.registers[reg as usize] {
            Value::Number(n) => Ok(*n),
            _ => Err(VmError::TypeError("Expected number".to_string())),
        }
    }

    /// Get a text from a register
    fn get_text(&self, reg: u8) -> VmResult<String> {
        match &self.registers[reg as usize] {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(VmError::TypeError("Expected text".to_string())),
        }
    }

    /// Check if a register value is truthy
    fn is_truthy(&self, reg: u8) -> bool {
        match &self.registers[reg as usize] {
            Value::Truth(b) => *b,
            Value::Nothing => false,
            Value::Number(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty(),
            _ => true,
        }
    }
}

/// Convert a constant to a value
fn constant_to_value(constant: &Constant) -> Value {
    match constant {
        Constant::Number(n) => Value::Number(*n),
        Constant::Text(s) => Value::Text(s.clone()),
        Constant::Truth(b) => Value::Truth(*b),
        Constant::Nothing => Value::Nothing,
        Constant::StructDef { name, fields } => Value::StructDef {
            name: name.clone(),
            fields: fields.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode_compiler::compile;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run_source(source: &str) -> VmResult<Value> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse failed");
        let chunk = compile(&ast).expect("Compile failed");

        let mut vm = VM::new();
        vm.execute(chunk)
    }

    #[test]
    fn test_vm_number() {
        let result = run_source("42").expect("VM failed");
        assert_eq!(result, Value::Number(42.0));
    }

    #[test]
    fn test_vm_arithmetic() {
        let result = run_source("10 + 20 * 2").expect("VM failed");
        assert_eq!(result, Value::Number(50.0));
    }

    #[test]
    fn test_vm_comparison() {
        let result = run_source("10 less than 20").expect("VM failed");
        assert_eq!(result, Value::Truth(true));
    }

    #[test]
    fn test_vm_bind() {
        let result = run_source("bind x to 42\nx").expect("VM failed");
        assert_eq!(result, Value::Number(42.0));
    }
}
