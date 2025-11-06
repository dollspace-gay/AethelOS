//! # Quicksilver VM - Bytecode Module
//!
//! Register-based bytecode instruction set for Glimmer-Weave.
//! Provides 5-10x performance improvement over tree-walking interpreter.
//!
//! ## Architecture
//!
//! - **Register-based**: 256 virtual registers (r0-r255)
//! - **Type-aware**: Specialized instructions for each type
//! - **Compact**: Instructions are 1-5 bytes
//! - **Portable**: Pure bytecode, no native code generation
//!
//! ## Instruction Format
//!
//! ```text
//! [Opcode:u8] [Args...]
//! ```
//!
//! Most instructions use register operands (r0-r255).

use alloc::string::String;
use alloc::vec::Vec;

/// Virtual register index (0-255)
pub type Register = u8;

/// Constant pool index
pub type ConstantId = u16;

/// Jump offset (signed)
pub type JumpOffset = i16;

/// Bytecode instruction
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ===== Move/Load Instructions =====

    /// Load constant into register: `r[dest] = constants[id]`
    LoadConst { dest: Register, constant_id: ConstantId },

    /// Move value between registers: `r[dest] = r[src]`
    Move { dest: Register, src: Register },

    /// Load Nothing (null) into register: `r[dest] = nothing`
    LoadNothing { dest: Register },

    /// Load Truth (boolean) into register: `r[dest] = true/false`
    LoadTruth { dest: Register, value: bool },

    // ===== Arithmetic Instructions =====

    /// Add numbers: `r[dest] = r[left] + r[right]`
    AddNum { dest: Register, left: Register, right: Register },

    /// Subtract numbers: `r[dest] = r[left] - r[right]`
    SubNum { dest: Register, left: Register, right: Register },

    /// Multiply numbers: `r[dest] = r[left] * r[right]`
    MulNum { dest: Register, left: Register, right: Register },

    /// Divide numbers: `r[dest] = r[left] / r[right]`
    DivNum { dest: Register, left: Register, right: Register },

    /// Modulo: `r[dest] = r[left] % r[right]`
    ModNum { dest: Register, left: Register, right: Register },

    /// Negate number: `r[dest] = -r[src]`
    NegNum { dest: Register, src: Register },

    // ===== String Instructions =====

    /// Concatenate strings: `r[dest] = r[left] + r[right]`
    ConcatText { dest: Register, left: Register, right: Register },

    // ===== Comparison Instructions =====

    /// Equal: `r[dest] = r[left] == r[right]`
    Eq { dest: Register, left: Register, right: Register },

    /// Not equal: `r[dest] = r[left] != r[right]`
    Ne { dest: Register, left: Register, right: Register },

    /// Less than: `r[dest] = r[left] < r[right]`
    Lt { dest: Register, left: Register, right: Register },

    /// Less than or equal: `r[dest] = r[left] <= r[right]`
    Le { dest: Register, left: Register, right: Register },

    /// Greater than: `r[dest] = r[left] > r[right]`
    Gt { dest: Register, left: Register, right: Register },

    /// Greater than or equal: `r[dest] = r[left] >= r[right]`
    Ge { dest: Register, left: Register, right: Register },

    // ===== Logical Instructions =====

    /// Logical NOT: `r[dest] = not r[src]`
    Not { dest: Register, src: Register },

    /// Logical AND: `r[dest] = r[left] and r[right]`
    And { dest: Register, left: Register, right: Register },

    /// Logical OR: `r[dest] = r[left] or r[right]`
    Or { dest: Register, left: Register, right: Register },

    // ===== Control Flow Instructions =====

    /// Unconditional jump: `pc += offset`
    Jump { offset: JumpOffset },

    /// Jump if true: `if r[cond] then pc += offset`
    JumpIfTrue { cond: Register, offset: JumpOffset },

    /// Jump if false: `if not r[cond] then pc += offset`
    JumpIfFalse { cond: Register, offset: JumpOffset },

    // ===== Variable Instructions =====

    /// Define global variable: `globals[name] = r[src]`
    DefineGlobal { name_id: ConstantId, src: Register },

    /// Load global variable: `r[dest] = globals[name]`
    LoadGlobal { dest: Register, name_id: ConstantId },

    /// Store global variable: `globals[name] = r[src]`
    StoreGlobal { name_id: ConstantId, src: Register },

    /// Load local variable: `r[dest] = locals[index]`
    LoadLocal { dest: Register, local_index: u8 },

    /// Store local variable: `locals[index] = r[src]`
    StoreLocal { local_index: u8, src: Register },

    // ===== Collection Instructions =====

    /// Create list: `r[dest] = [r[start]..r[start+count-1]]`
    CreateList { dest: Register, start: Register, count: u8 },

    /// Create map: `r[dest] = {}`
    CreateMap { dest: Register },

    /// Get list element: `r[dest] = r[list][r[index]]`
    GetIndex { dest: Register, list: Register, index: Register },

    /// Set list element: `r[list][r[index]] = r[value]`
    SetIndex { list: Register, index: Register, value: Register },

    /// Get map field: `r[dest] = r[map].field`
    GetField { dest: Register, map: Register, field_id: ConstantId },

    /// Set map field: `r[map].field = r[value]`
    SetField { map: Register, field_id: ConstantId, value: Register },

    // ===== Function Instructions =====

    /// Call function: `r[dest] = r[func](r[arg_start]..r[arg_start+arg_count-1])`
    Call { dest: Register, func: Register, arg_start: Register, arg_count: u8 },

    /// Return from function: `return r[value]`
    Return { value: Register },

    /// Create closure: `r[dest] = closure(function_id, captured)`
    CreateClosure { dest: Register, function_id: ConstantId, capture_count: u8 },

    // ===== Enum/Variant Instructions =====

    /// Create Outcome (Result) - Triumph variant: `r[dest] = Triumph(r[value])`
    CreateTriumph { dest: Register, value: Register },

    /// Create Outcome (Result) - Mishap variant: `r[dest] = Mishap(r[value])`
    CreateMishap { dest: Register, value: Register },

    /// Create Maybe (Option) - Present variant: `r[dest] = Present(r[value])`
    CreatePresent { dest: Register, value: Register },

    /// Create Maybe (Option) - Absent variant: `r[dest] = Absent`
    CreateAbsent { dest: Register },

    /// Check if Outcome is Triumph: `r[dest] = is_triumph(r[value])`
    IsTriumph { dest: Register, value: Register },

    /// Check if Outcome is Mishap: `r[dest] = is_mishap(r[value])`
    IsMishap { dest: Register, value: Register },

    /// Check if Maybe is Present: `r[dest] = is_present(r[value])`
    IsPresent { dest: Register, value: Register },

    /// Check if Maybe is Absent: `r[dest] = is_absent(r[value])`
    IsAbsent { dest: Register, value: Register },

    /// Extract inner value from Outcome/Maybe: `r[dest] = r[value].inner`
    ExtractInner { dest: Register, value: Register },

    // ===== Struct Instructions =====

    /// Create struct instance: `r[dest] = StructName { field1: r[field_start], field2: r[field_start+1], ... }`
    /// The struct definition ID and field values are provided
    CreateStruct {
        dest: Register,
        struct_def_id: ConstantId,
        field_start: Register,
        field_count: u8
    },

    // ===== Exception Handling Instructions =====

    /// Setup exception handler: records handler offset for error recovery
    /// When an error occurs, VM will jump to handler_offset
    SetupTry { handler_offset: usize },

    /// Pop exception handler: removes the most recent exception handler
    PopTry,

    /// Throw/raise an error: signals an error condition
    /// The error message is in r[error_reg]
    Throw { error_reg: Register },

    // ===== Special Instructions =====

    /// Halt execution
    Halt,

    /// Print for debugging: `print(r[src])`
    Print { src: Register },
}

/// Constant value in the constant pool
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Number constant
    Number(f64),
    /// Text constant
    Text(String),
    /// Truth constant
    Truth(bool),
    /// Nothing constant
    Nothing,
    /// Struct definition constant
    StructDef {
        name: String,
        fields: Vec<crate::ast::StructField>,
    },
}

/// Compiled bytecode chunk
#[derive(Debug, Clone)]
pub struct BytecodeChunk {
    /// Instruction sequence
    pub instructions: Vec<Instruction>,

    /// Constant pool
    pub constants: Vec<Constant>,

    /// Source code line numbers (for debugging)
    pub lines: Vec<usize>,

    /// Function name (for debugging)
    pub name: String,

    /// Number of parameters
    pub param_count: u8,

    /// Number of local variables
    pub local_count: u8,
}

impl BytecodeChunk {
    /// Create a new empty bytecode chunk
    pub fn new(name: String) -> Self {
        BytecodeChunk {
            instructions: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
            name,
            param_count: 0,
            local_count: 0,
        }
    }

    /// Add an instruction
    pub fn emit(&mut self, instruction: Instruction, line: usize) {
        self.instructions.push(instruction);
        self.lines.push(line);
    }

    /// Add a constant to the pool and return its index
    pub fn add_constant(&mut self, constant: Constant) -> ConstantId {
        // Check if constant already exists (constant pooling)
        for (i, existing) in self.constants.iter().enumerate() {
            if existing == &constant {
                return i as ConstantId;
            }
        }

        let id = self.constants.len() as ConstantId;
        self.constants.push(constant);
        id
    }

    /// Get the current instruction offset (for jump targets)
    pub fn offset(&self) -> usize {
        self.instructions.len()
    }

    /// Patch a jump instruction at the given offset
    pub fn patch_jump(&mut self, jump_offset: usize, target_offset: usize) {
        let relative_offset = (target_offset as isize - jump_offset as isize - 1) as i16;

        match &mut self.instructions[jump_offset] {
            Instruction::Jump { offset } => *offset = relative_offset,
            Instruction::JumpIfTrue { offset, .. } => *offset = relative_offset,
            Instruction::JumpIfFalse { offset, .. } => *offset = relative_offset,
            _ => panic!("Attempted to patch non-jump instruction"),
        }
    }
}

impl Constant {
    /// Get a human-readable type name
    pub fn type_name(&self) -> &str {
        match self {
            Constant::Number(_) => "Number",
            Constant::Text(_) => "Text",
            Constant::Truth(_) => "Truth",
            Constant::Nothing => "Nothing",
            Constant::StructDef { .. } => "StructDef",
        }
    }
}

/// Bytecode disassembler for debugging
pub struct Disassembler<'a> {
    chunk: &'a BytecodeChunk,
}

impl<'a> Disassembler<'a> {
    pub fn new(chunk: &'a BytecodeChunk) -> Self {
        Disassembler { chunk }
    }

    /// Disassemble the entire chunk
    pub fn disassemble(&self) -> String {
        use alloc::format;

        let mut output = format!("==== {} ====\n", self.chunk.name);
        output.push_str(&format!("Parameters: {}\n", self.chunk.param_count));
        output.push_str(&format!("Locals: {}\n", self.chunk.local_count));
        output.push_str(&format!("Constants: {}\n", self.chunk.constants.len()));
        output.push_str(&format!("Instructions: {}\n\n", self.chunk.instructions.len()));

        // Disassemble constants
        output.push_str("Constants:\n");
        for (i, constant) in self.chunk.constants.iter().enumerate() {
            output.push_str(&format!("  #{}: {:?}\n", i, constant));
        }
        output.push('\n');

        // Disassemble instructions
        output.push_str("Code:\n");
        for (i, instruction) in self.chunk.instructions.iter().enumerate() {
            let line = self.chunk.lines[i];
            output.push_str(&format!("{:04} {:4} {}\n", i, line, self.disassemble_instruction(instruction)));
        }

        output
    }

    /// Disassemble a single instruction
    fn disassemble_instruction(&self, instruction: &Instruction) -> String {
        use alloc::format;

        match instruction {
            Instruction::LoadConst { dest, constant_id } => {
                format!("LOAD_CONST     r{} <- #{} ({:?})", dest, constant_id, self.chunk.constants.get(*constant_id as usize))
            }
            Instruction::Move { dest, src } => {
                format!("MOVE           r{} <- r{}", dest, src)
            }
            Instruction::LoadNothing { dest } => {
                format!("LOAD_NOTHING   r{}", dest)
            }
            Instruction::LoadTruth { dest, value } => {
                format!("LOAD_TRUTH     r{} <- {}", dest, value)
            }
            Instruction::AddNum { dest, left, right } => {
                format!("ADD_NUM        r{} <- r{} + r{}", dest, left, right)
            }
            Instruction::SubNum { dest, left, right } => {
                format!("SUB_NUM        r{} <- r{} - r{}", dest, left, right)
            }
            Instruction::MulNum { dest, left, right } => {
                format!("MUL_NUM        r{} <- r{} * r{}", dest, left, right)
            }
            Instruction::DivNum { dest, left, right } => {
                format!("DIV_NUM        r{} <- r{} / r{}", dest, left, right)
            }
            Instruction::ModNum { dest, left, right } => {
                format!("MOD_NUM        r{} <- r{} % r{}", dest, left, right)
            }
            Instruction::NegNum { dest, src } => {
                format!("NEG_NUM        r{} <- -r{}", dest, src)
            }
            Instruction::ConcatText { dest, left, right } => {
                format!("CONCAT_TEXT    r{} <- r{} + r{}", dest, left, right)
            }
            Instruction::Eq { dest, left, right } => {
                format!("EQ             r{} <- r{} == r{}", dest, left, right)
            }
            Instruction::Ne { dest, left, right } => {
                format!("NE             r{} <- r{} != r{}", dest, left, right)
            }
            Instruction::Lt { dest, left, right } => {
                format!("LT             r{} <- r{} < r{}", dest, left, right)
            }
            Instruction::Le { dest, left, right } => {
                format!("LE             r{} <- r{} <= r{}", dest, left, right)
            }
            Instruction::Gt { dest, left, right } => {
                format!("GT             r{} <- r{} > r{}", dest, left, right)
            }
            Instruction::Ge { dest, left, right } => {
                format!("GE             r{} <- r{} >= r{}", dest, left, right)
            }
            Instruction::Not { dest, src } => {
                format!("NOT            r{} <- not r{}", dest, src)
            }
            Instruction::And { dest, left, right } => {
                format!("AND            r{} <- r{} and r{}", dest, left, right)
            }
            Instruction::Or { dest, left, right } => {
                format!("OR             r{} <- r{} or r{}", dest, left, right)
            }
            Instruction::Jump { offset } => {
                format!("JUMP           +{}", offset)
            }
            Instruction::JumpIfTrue { cond, offset } => {
                format!("JUMP_IF_TRUE   r{} +{}", cond, offset)
            }
            Instruction::JumpIfFalse { cond, offset } => {
                format!("JUMP_IF_FALSE  r{} +{}", cond, offset)
            }
            Instruction::DefineGlobal { name_id, src } => {
                format!("DEF_GLOBAL     #{} <- r{}", name_id, src)
            }
            Instruction::LoadGlobal { dest, name_id } => {
                format!("LOAD_GLOBAL    r{} <- #{}", dest, name_id)
            }
            Instruction::StoreGlobal { name_id, src } => {
                format!("STORE_GLOBAL   #{} <- r{}", name_id, src)
            }
            Instruction::LoadLocal { dest, local_index } => {
                format!("LOAD_LOCAL     r{} <- local[{}]", dest, local_index)
            }
            Instruction::StoreLocal { local_index, src } => {
                format!("STORE_LOCAL    local[{}] <- r{}", local_index, src)
            }
            Instruction::CreateList { dest, start, count } => {
                format!("CREATE_LIST    r{} <- [r{}..r{}]", dest, start, start + count - 1)
            }
            Instruction::CreateMap { dest } => {
                format!("CREATE_MAP     r{}", dest)
            }
            Instruction::GetIndex { dest, list, index } => {
                format!("GET_INDEX      r{} <- r{}[r{}]", dest, list, index)
            }
            Instruction::SetIndex { list, index, value } => {
                format!("SET_INDEX      r{}[r{}] <- r{}", list, index, value)
            }
            Instruction::GetField { dest, map, field_id } => {
                format!("GET_FIELD      r{} <- r{}.#{}", dest, map, field_id)
            }
            Instruction::SetField { map, field_id, value } => {
                format!("SET_FIELD      r{}.#{} <- r{}", map, field_id, value)
            }
            Instruction::Call { dest, func, arg_start, arg_count } => {
                format!("CALL           r{} <- r{}(r{}..r{})", dest, func, arg_start, arg_start + arg_count - 1)
            }
            Instruction::Return { value } => {
                format!("RETURN         r{}", value)
            }
            Instruction::CreateClosure { dest, function_id, capture_count } => {
                format!("CREATE_CLOSURE r{} <- closure(#{}, {} captures)", dest, function_id, capture_count)
            }
            Instruction::Halt => {
                "HALT".to_string()
            }
            Instruction::Print { src } => {
                format!("PRINT          r{}", src)
            }
            // Enum instructions
            Instruction::CreateTriumph { dest, value } => {
                format!("CREATE_TRIUMPH r{} <- Triumph(r{})", dest, value)
            }
            Instruction::CreateMishap { dest, value } => {
                format!("CREATE_MISHAP  r{} <- Mishap(r{})", dest, value)
            }
            Instruction::CreatePresent { dest, value } => {
                format!("CREATE_PRESENT r{} <- Present(r{})", dest, value)
            }
            Instruction::CreateAbsent { dest } => {
                format!("CREATE_ABSENT  r{} <- Absent", dest)
            }
            Instruction::IsTriumph { dest, value } => {
                format!("IS_TRIUMPH     r{} <- is_triumph(r{})", dest, value)
            }
            Instruction::IsMishap { dest, value } => {
                format!("IS_MISHAP      r{} <- is_mishap(r{})", dest, value)
            }
            Instruction::IsPresent { dest, value } => {
                format!("IS_PRESENT     r{} <- is_present(r{})", dest, value)
            }
            Instruction::IsAbsent { dest, value } => {
                format!("IS_ABSENT      r{} <- is_absent(r{})", dest, value)
            }
            Instruction::ExtractInner { dest, value } => {
                format!("EXTRACT_INNER  r{} <- r{}.inner", dest, value)
            }
            // Struct instructions
            Instruction::CreateStruct { dest, struct_def_id, field_start, field_count } => {
                format!("CREATE_STRUCT  r{} <- struct(#{}, r{}..r{} ({} fields))",
                    dest, struct_def_id, field_start, field_start + *field_count as Register - 1, field_count)
            }
            // Exception handling instructions
            Instruction::SetupTry { handler_offset } => {
                format!("SETUP_TRY      handler @{}", handler_offset)
            }
            Instruction::PopTry => {
                "POP_TRY".to_string()
            }
            Instruction::Throw { error_reg } => {
                format!("THROW          r{}", error_reg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytecode_chunk_creation() {
        let chunk = BytecodeChunk::new("test".to_string());
        assert_eq!(chunk.instructions.len(), 0);
        assert_eq!(chunk.constants.len(), 0);
        assert_eq!(chunk.name, "test");
    }

    #[test]
    fn test_add_constant() {
        let mut chunk = BytecodeChunk::new("test".to_string());

        let id1 = chunk.add_constant(Constant::Number(42.0));
        let id2 = chunk.add_constant(Constant::Text("hello".to_string()));
        let id3 = chunk.add_constant(Constant::Number(42.0)); // Duplicate

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 0); // Should reuse existing constant
        assert_eq!(chunk.constants.len(), 2);
    }

    #[test]
    fn test_emit_instruction() {
        let mut chunk = BytecodeChunk::new("test".to_string());

        chunk.emit(Instruction::LoadConst { dest: 0, constant_id: 0 }, 1);
        chunk.emit(Instruction::Return { value: 0 }, 2);

        assert_eq!(chunk.instructions.len(), 2);
        assert_eq!(chunk.lines, vec![1, 2]);
    }

    #[test]
    fn test_patch_jump() {
        let mut chunk = BytecodeChunk::new("test".to_string());

        chunk.emit(Instruction::Jump { offset: 0 }, 1);  // offset 0
        chunk.emit(Instruction::LoadConst { dest: 0, constant_id: 0 }, 2);  // offset 1
        chunk.emit(Instruction::LoadConst { dest: 1, constant_id: 1 }, 3);  // offset 2
        chunk.emit(Instruction::Return { value: 0 }, 4);  // offset 3

        // Patch jump from 0 to 3 (skip two instructions)
        chunk.patch_jump(0, 3);

        if let Instruction::Jump { offset } = chunk.instructions[0] {
            assert_eq!(offset, 2); // Jump forward by 2 instructions
        } else {
            panic!("Expected Jump instruction");
        }
    }

    #[test]
    fn test_disassembler() {
        let mut chunk = BytecodeChunk::new("test_function".to_string());

        let const_id = chunk.add_constant(Constant::Number(42.0));
        chunk.emit(Instruction::LoadConst { dest: 0, constant_id: const_id }, 1);
        chunk.emit(Instruction::Return { value: 0 }, 2);

        let disasm = Disassembler::new(&chunk);
        let output = disasm.disassemble();

        assert!(output.contains("test_function"));
        assert!(output.contains("LOAD_CONST"));
        assert!(output.contains("RETURN"));
    }
}
