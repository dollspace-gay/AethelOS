# CLAUDE.md - AI Assistant Guide to Glimmer-Weave Development

> **Quick Reference:** This document is designed for AI assistants (Claude, GPT, etc.) working on Glimmer-Weave, the scripting language for AethelOS. It provides build commands, design philosophy, coding standards, and roadmap for achieving "full Rust strength" in language features.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Issue Tracking with Beads](#issue-tracking-with-beads)
3. [Design Philosophy](#design-philosophy)
4. [Architecture Overview](#architecture-overview)
5. [Coding Standards & Quality Controls](#coding-standards--quality-controls)
6. [Current Implementation Status](#current-implementation-status)
7. [Roadmap to Rust-Level Features](#roadmap-to-rust-level-features)
8. [Common Tasks](#common-tasks)
9. [Testing Strategy](#testing-strategy)

---

## Quick Start

### About Glimmer-Weave

**Glimmer-Weave** is a modern scripting language designed for AethelOS but being developed as a **system-agnostic** language that can run anywhere. It emphasizes:

- **Natural language syntax** - Readable, expressive keywords over terse syntax
- **Rust-inspired features** - Memory safety, pattern matching, Result/Option types
- **Multiple execution engines** - Interpreter, bytecode VM, and native x86-64 codegen
- **No runtime dependencies** - `no_std` compatible for embedded/OS development

### Build Commands

**From groves/glimmer_weave directory:**

```bash
# 1. Build the library
cargo build

# 2. Run all tests (interpreter, bytecode VM, parser)
cargo test

# 3. Run specific test suites
cargo test --lib                    # Library tests only
cargo test --test integration_test  # Integration tests

# 4. Check for errors without building
cargo check

# 5. Run clippy linter
cargo clippy

# 6. Build documentation
cargo doc --open
```

### Project Structure

```
groves/glimmer_weave/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── token.rs            # Token definitions
│   ├── lexer.rs            # Tokenizer
│   ├── ast.rs              # Abstract syntax tree
│   ├── parser.rs           # Parser (tokens → AST)
│   ├── eval.rs             # Tree-walking interpreter
│   ├── semantic.rs         # Semantic analysis / type checking
│   ├── bytecode.rs         # Bytecode instruction set
│   ├── bytecode_compiler.rs # Compiler (AST → bytecode)
│   ├── vm.rs               # Bytecode virtual machine
│   ├── codegen.rs          # Native x86-64 code generator
│   └── runtime.rs          # Native runtime functions
├── examples/               # Example .gw programs
├── tests/                  # Integration tests
└── CLAUDE.md              # This file
```

---

## Issue Tracking with Beads

### CRITICAL: Use bd for ALL Work Tracking

**For all work in this repository, you MUST use the beads issue tracker.**

- Use the `bd` command-line tool to create, manage, and close issues
- **DO NOT** use markdown files for creating to-do lists
- **DO NOT** use the TodoWrite tool for tracking work across sessions
- All issues and bugs are to be tracked via `bd`

### bd - Dependency-Aware Issue Tracker

Issues chained together like beads.

#### Getting Started

```bash
# Initialize bd in the project (if not already initialized)
bd init

# Initialize with custom prefix
bd init --prefix gw
# Issues will be named: gw-1, gw-2, etc.
```

#### Creating Issues

```bash
# Create a simple issue
bd create "Add closure support to bytecode VM"

# Create with priority (0=highest, 4=lowest)
bd create "Implement generics" -p 0 -t feature

# Create with description
bd create "Fix parser error on nested structs" -d "Parser fails when structs contain other structs"
```

#### Viewing Issues

```bash
# List all issues
bd list

# List by status
bd list --status open
bd list --status in_progress
bd list --status completed

# List by priority (0=highest)
bd list --priority 0

# Show detailed issue info
bd show gw-1
```

#### Managing Dependencies

```bash
# Add dependency (gw-2 blocks gw-1)
bd dep add gw-1 gw-2

# Visualize dependency tree
bd dep tree gw-1

# Check for circular dependencies
bd dep cycles
```

**Dependency Types:**
- `blocks` - Task B must complete before task A
- `related` - Soft connection, doesn't block progress
- `parent-child` - Epic/subtask hierarchical relationship
- `discovered-from` - Auto-created when AI discovers related work

#### Finding Ready Work

```bash
# Show issues ready to work on
bd ready
# Ready = status is 'open' AND no blocking dependencies
# Perfect for agents to claim next work!
```

#### Updating Issues

```bash
# Update status
bd update gw-1 --status in_progress
bd update gw-1 --status completed

# Update priority
bd update gw-1 --priority 0

# Assign to someone
bd update gw-1 --assignee alice
```

#### Closing Issues

```bash
# Close a single issue
bd close gw-1

# Close multiple issues
bd close gw-2 gw-3 --reason "Implemented in PR #42"
```

#### Git Workflow (Auto-Sync)

bd automatically keeps git in sync:
- ✓ Export to JSONL after CRUD operations (5s debounce)
- ✓ Import from JSONL when newer than DB (after git pull)
- ✓ Works seamlessly across machines and team members
- **No manual export/import needed!**

Disable with: `--no-auto-flush` or `--no-auto-import`

#### Database Location

bd automatically discovers your database:
1. `--db /path/to/db.db` flag
2. `$BEADS_DB` environment variable
3. `.beads/*.db` in current directory or ancestors
4. `~/.beads/default.db` as fallback

---

## Design Philosophy

### Core Principle: Natural Expression

> *"The code should read like poetry, not algebra."*

Glimmer-Weave is **not** a clone of Python, JavaScript, or even Rust syntax. It is a principled rethinking of how code should look and feel:

### 1. **Readability Over Brevity**

- **NOT:** `let mut x = 5;`
- **BUT:** `weave counter as 5` (mutable) or `bind value to 42` (immutable)
- **METAPHOR:** Weaving threads together vs. declaring variables

**Examples:**

```glimmer-weave
# Variables
bind name to "Elara"           # Immutable binding
weave counter as 0             # Mutable variable
set counter to counter + 1     # Mutation

# Control flow
should age >= 18 then
    VGA.write("Welcome")
otherwise
    VGA.write("Access denied")
end

# Loops
for each item in items then
    process(item)
end

# Functions
chant factorial(n) then
    should n <= 1 then
        yield 1
    otherwise
        yield n * factorial(n - 1)
    end
end
```

### 2. **Rust-Inspired Safety**

Glimmer-Weave inherits Rust's best practices:

- **No null pointers** - Use `Maybe` (Option) type: `Present(value)` or `Absent`
- **Explicit error handling** - Use `Outcome` (Result) type: `Triumph(value)` or `Mishap(error)`
- **Pattern matching** - Exhaustive `match` expressions
- **Immutable by default** - Use `bind` for constants, `weave` for mutables
- **Strong typing** - Type annotations optional but encouraged

**Examples:**

```glimmer-weave
# Result/Outcome type
chant divide(a, b) then
    should b is 0 then
        yield Mishap("Division by zero")
    otherwise
        yield Triumph(a / b)
    end
end

bind result to divide(10, 2)
match result with
    when Triumph(value) then
        VGA.write("Success: " + to_text(value))
    when Mishap(err) then
        VGA.write("Error: " + err)
end

# Option/Maybe type
chant find_first(list, predicate) then
    for each item in list then
        should predicate(item) then
            yield Present(item)
        end
    end
    yield Absent
end

bind found to find_first([1, 2, 3], |x| x > 2)
match found with
    when Present(value) then VGA.write("Found: " + to_text(value))
    when Absent then VGA.write("Not found")
end
```

### 3. **Error Handling Without Exceptions**

- **NOT:** Try/catch that can be forgotten
- **BUT:** `attempt/harmonize` blocks with explicit error types
- **METAPHOR:** Harmonizing discordant notes vs. catching exceptions

**Example:**

```glimmer-weave
attempt then
    bind result to risky_operation()
    VGA.write("Success: " + result)
harmonize on "NetworkError" then
    VGA.write("Network failed, retrying...")
    retry_operation()
harmonize on _ then
    VGA.write("Unknown error occurred")
end
```

### 4. **Custom Data Types**

- **Structs** - Named, typed fields using `form` keyword
- **Pattern matching** - Destructure and match on types
- **Composition** - Build complex types from simple ones

**Example:**

```glimmer-weave
form Point with
    x as Number
    y as Number
end

form Rectangle with
    top_left as Point
    bottom_right as Point
end

bind rect to Rectangle {
    top_left: Point { x: 0, y: 10 },
    bottom_right: Point { x: 5, y: 0 }
}

bind width to rect.bottom_right.x - rect.top_left.x
```

---

## Architecture Overview

### Execution Engines

Glimmer-Weave has **three execution modes**, each with different trade-offs:

```
┌─────────────────────────────────────────────────────────┐
│                   Source Code (.gw)                      │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              Lexer (token.rs, lexer.rs)                  │
│         Text → Tokens (keywords, identifiers, etc.)      │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│              Parser (parser.rs, ast.rs)                  │
│         Tokens → Abstract Syntax Tree (AST)              │
└─────────────────────────────────────────────────────────┘
                          ↓
              ┌───────────┴───────────┬──────────────┐
              ↓                       ↓              ↓
┌─────────────────────┐  ┌────────────────────┐  ┌──────────────────┐
│  Tree-Walking       │  │  Bytecode          │  │  Native x86-64   │
│  Interpreter        │  │  Compiler + VM     │  │  Code Generator  │
│  (eval.rs)          │  │  (bytecode_*.rs)   │  │  (codegen.rs)    │
├─────────────────────┤  ├────────────────────┤  ├──────────────────┤
│ ✅ Full features    │  │ ✅ Full features   │  │ ⚠️  Limited      │
│ 🐌 Slowest          │  │ ⚡ Fast            │  │ ⚡⚡ Fastest      │
│ 🧪 Best for dev     │  │ 🎯 Production use  │  │ 🎯 AOT compile   │
│ 💾 High memory      │  │ 💾 Low memory      │  │ 💾 No runtime    │
└─────────────────────┘  └────────────────────┘  └──────────────────┘
```

### Component Status

| Component | Purpose | Status | Notes |
|-----------|---------|--------|-------|
| **Lexer** | Tokenization | ✅ Complete | All keywords recognized |
| **Parser** | AST generation | ✅ Complete | All syntax parsed |
| **Interpreter** | Direct AST execution | ✅ Complete | Full feature support |
| **Semantic Analyzer** | Type checking | 🟡 Partial | Basic checks only |
| **Bytecode Compiler** | AST → bytecode | ✅ Complete | All features compiled |
| **Bytecode VM** | Execute bytecode | ✅ Complete | Register-based VM |
| **Native Codegen** | x86-64 assembly | 🟡 Partial | No heap allocation yet |
| **Runtime Library** | Native functions | 🟡 Partial | Basic math/string ops |

**Status Legend:**
- ✅ **Complete:** Fully implemented and tested
- 🟡 **Partial:** Working but incomplete
- ⚪ **Planned:** Not started

---

## Coding Standards & Quality Controls

### 🔴 CRITICAL RULES - MUST FOLLOW

#### 1. **Maintain Three Execution Engines in Sync**

When adding a new language feature, you MUST implement it in **all three engines**:

```rust
// 1. Add to AST (ast.rs)
pub enum AstNode {
    // ... existing variants
    MyNewFeature { field: String },
}

// 2. Add to parser (parser.rs)
fn parse_statement(&mut self) -> ParseResult<AstNode> {
    match self.current() {
        Token::MyKeyword => self.parse_my_feature(),
        // ...
    }
}

// 3. Add to interpreter (eval.rs)
fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
    match node {
        AstNode::MyNewFeature { field } => {
            // Implement feature
        }
        // ...
    }
}

// 4. Add to bytecode compiler (bytecode_compiler.rs)
fn compile_stmt(&mut self, node: &AstNode) -> CompileResult<Option<Register>> {
    match node {
        AstNode::MyNewFeature { field } => {
            // Compile to bytecode
        }
        // ...
    }
}

// 5. Add to VM (vm.rs) if new instruction needed
match instruction {
    Instruction::MyNewInstruction { dest, src } => {
        // Execute instruction
    }
    // ...
}

// 6. Add to codegen (codegen.rs) OR document limitation
fn gen_statement(&mut self, node: &AstNode) -> Result<(), String> {
    match node {
        AstNode::MyNewFeature { field } => {
            // Either: Generate x86-64 code
            // OR: Return error with clear message directing to interpreter/VM
        }
        // ...
    }
}
```

**Rule:** Never leave an AST variant unhandled. Use exhaustive `match` statements. The compiler will catch this with an error.

#### 2. **No Panics in Core Language Execution**

```rust
// ❌ WRONG - Panicking in evaluator
fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
    match node {
        AstNode::Divide { left, right } => {
            let r = self.eval_node(right)?;
            assert!(r != 0);  // NEVER DO THIS!
        }
    }
}

// ✅ CORRECT - Return error
fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
    match node {
        AstNode::BinaryOp { left, op: BinaryOperator::Div, right } => {
            let l = self.eval_node(left)?;
            let r = self.eval_node(right)?;
            if r == 0.0 {
                return Err(RuntimeError::Custom("Division by zero".to_string()));
            }
            Ok(Value::Number(l / r))
        }
    }
}
```

**Rule:** Use `Result<T, E>` for all operations that can fail. Never use `unwrap()`, `expect()`, or `assert!()` in hot paths.

#### 3. **Preserve Natural Language Naming**

```rust
// ❌ WRONG - Generic/boring names
enum Token {
    Let,
    Var,
    Return,
    Try,
    Catch,
}

// ✅ CORRECT - Natural, poetic names
enum Token {
    Bind,      // Immutable binding
    Weave,     // Mutable variable
    Yield,     // Return from function
    Attempt,   // Try block
    Harmonize, // Catch/error handler
}
```

**Naming Conventions:**

| Concept | Glimmer-Weave | Traditional |
|---------|---------------|-------------|
| Immutable var | `bind x to 5` | `let x = 5` |
| Mutable var | `weave x as 5` | `let mut x = 5` |
| Assignment | `set x to 10` | `x = 10` |
| Function | `chant add(a, b)` | `fn add(a, b)` |
| Return | `yield result` | `return result` |
| If/else | `should...then...otherwise` | `if...else` |
| For loop | `for each x in list then` | `for x in list` |
| While loop | `whilst condition then` | `while condition` |
| Struct | `form Point with x as Number` | `struct Point { x: i32 }` |
| Result Ok | `Triumph(value)` | `Ok(value)` |
| Result Err | `Mishap(error)` | `Err(error)` |
| Option Some | `Present(value)` | `Some(value)` |
| Option None | `Absent` | `None` |
| Try/catch | `attempt...harmonize` | `try...catch` |
| Match | `match...when` | `match...=>`  |

**Rule:** All new keywords must follow the natural language philosophy. When in doubt, choose the word that reads like English prose.

#### 4. **Comprehensive Testing**

Every new feature MUST have tests in **all applicable engines**:

```rust
// In eval.rs tests
#[test]
fn test_my_feature_interpreter() {
    let source = r#"
        # Test my new feature
        bind x to my_feature(42)
    "#;
    let result = eval_source(source).expect("Eval failed");
    assert_eq!(result, Value::Number(42.0));
}

// In bytecode_compiler.rs tests
#[test]
fn test_my_feature_bytecode() {
    let chunk = compile_source(r#"
        bind x to my_feature(42)
    "#).expect("Compile failed");

    // Verify instructions were emitted
    assert!(chunk.instructions.iter().any(|inst| {
        matches!(inst, Instruction::MyFeatureInstruction { .. })
    }));
}

// In integration tests
#[test]
fn test_my_feature_vm_execution() {
    let source = r#"
        bind x to my_feature(42)
    "#;
    let result = run_in_vm(source).expect("VM execution failed");
    assert_eq!(result, Value::Number(42.0));
}
```

**Rule:** Minimum 3 tests per feature: interpreter, bytecode compilation, VM execution.

#### 5. **Document Limitations Clearly**

If a feature cannot be implemented in native codegen due to missing runtime support:

```rust
// In codegen.rs
AstNode::StructLiteral { struct_name, fields: _ } => {
    // Struct instantiation requires heap allocation runtime
    self.emit(Instruction::Comment(format!("Struct literal: {}", struct_name)));
    self.emit(Instruction::Comment("Note: Struct instantiation requires heap allocation runtime".to_string()));
    self.emit(Instruction::Comment("This feature is fully supported in interpreter and bytecode VM".to_string()));

    // Return clear error
    Err(format!("Struct literals not supported in native codegen (requires heap allocation runtime). Use interpreter or bytecode VM instead."))
}
```

**Rule:** Never silently skip features. Always emit comments + error messages explaining:
- What's missing
- Why it's not implemented
- Where to use the feature instead

### 🟡 IMPORTANT GUIDELINES

#### Error Messages

```rust
// Provide helpful error messages
return Err(RuntimeError::Custom(format!(
    "Cannot access field '{}' on type {}. Did you mean '{}'?",
    field, type_name, suggestion
)));

// Not just:
return Err(RuntimeError::Custom("Field not found".to_string()));
```

#### Documentation

```rust
/// Evaluates a binary operation on two values.
///
/// # Arguments
/// * `left` - Left operand (already evaluated)
/// * `op` - Binary operator (Add, Sub, Mul, Div, Mod, etc.)
/// * `right` - Right operand (already evaluated)
///
/// # Returns
/// * `Ok(Value)` - Result of the operation
/// * `Err(RuntimeError)` - If operation is invalid (e.g., division by zero)
///
/// # Example
/// ```
/// let result = eval_binary_op(Value::Number(10.0), BinaryOperator::Add, Value::Number(5.0))?;
/// assert_eq!(result, Value::Number(15.0));
/// ```
fn eval_binary_op(&mut self, left: Value, op: BinaryOperator, right: Value) -> Result<Value, RuntimeError>
```

**Rule:** Public functions and complex private functions MUST have doc comments.

#### Performance Comments

```rust
// PERF: Using BTreeMap instead of HashMap because we need ordered iteration
// for consistent field access in structs
use alloc::collections::BTreeMap;

// PERF: Caching the struct definition lookup to avoid repeated global lookups
// in tight loops. Benchmarks show 3x speedup for struct-heavy code.
let cached_def = self.struct_cache.get(struct_name);
```

**Rule:** Non-obvious performance decisions should be documented with `// PERF:` comments.

---

## Current Implementation Status

### ✅ Fully Implemented Features

#### Core Language

- **Variables**
  - Immutable bindings: `bind x to 42`
  - Mutable variables: `weave counter as 0`
  - Mutation: `set counter to counter + 1`

- **Control Flow**
  - Conditionals: `should...then...otherwise...end`
  - For-each loops: `for each item in list then...end`
  - While loops: `whilst condition then...end`
  - Ranges: `range(1, 10)`

- **Functions**
  - Definition: `chant add(a, b) then...end`
  - Return: `yield result`
  - Recursion: Fully supported
  - Tail call optimization: Implemented in bytecode VM and native codegen
  - Closures: Functions capture their environment

- **Data Structures**
  - Numbers: `42`, `3.14`
  - Text (strings): `"Hello, World!"`
  - Truth (booleans): `true`, `false`
  - Nothing (null): `nothing`
  - Lists: `[1, 2, 3]`
  - Maps: `{name: "Elara", age: 42}`
  - Structs: `form Person with name as Text age as Number end`

- **Operators**
  - Arithmetic: `+`, `-`, `*`, `/`, `%`
  - Comparison: `>`, `<`, `>=`, `<=`, `is` (==), `is not` (!=)
  - Logical: `and`, `or`, `not`

#### Advanced Features

- **Pattern Matching**
  - `match...when` expressions
  - Literal patterns: `when 42 then...`
  - Variable binding: `when x then...` (binds matched value)
  - Wildcard: `when _ then...`
  - Enum patterns: `when Triumph(value) then...`
  - Exhaustiveness checking (via semantic analyzer)

- **Error Handling**
  - `attempt...harmonize` blocks
  - Error type matching: `harmonize on "NetworkError" then...`
  - Wildcard handler: `harmonize on _ then...`
  - Outcome type: `Triumph(value)` / `Mishap(error)`
  - Maybe type: `Present(value)` / `Absent`

- **Custom Types**
  - Struct definitions: `form Name with...end`
  - Struct instantiation: `Person { name: "Alice", age: 30 }`
  - Field access: `person.name`, `rect.top_left.x`
  - Nested structs: Fully supported

### 🟡 Partially Implemented

- **Type System**
  - Type annotations: Parsed but not fully enforced
  - Type inference: Basic support
  - Type checking: Only in semantic analyzer (not enforced at runtime)
  - Generics: Not yet implemented

- **Native Codegen**
  - Basic arithmetic: ✅
  - Control flow: ✅
  - Functions: ✅
  - Tail call optimization: ✅
  - Enums (Outcome/Maybe): ✅
  - Structs: ⚠️ Requires heap allocation runtime (not implemented)
  - Closures: ⚠️ Requires heap allocation

- **Standard Library**
  - Runtime functions available: `to_text`, `length`, `push`, `pop`, etc.
  - Limited set compared to production languages
  - No filesystem, network, or OS integration yet

### ⚪ Not Yet Implemented

- **Planned Language Features**
  - Generics / parametric polymorphism
  - Traits / interfaces
  - Modules / namespaces
  - Imports / exports
  - Async / await
  - Pipelines: `value | function1 | function2`
  - Capability requests: `request FileAccess with justification "logging"`

- **Advanced Runtime Features**
  - Heap allocation (malloc/free)
  - Garbage collection or reference counting
  - Dynamic dispatch
  - Reflection / introspection

- **Tooling**
  - REPL (Read-Eval-Print Loop)
  - Package manager
  - Build system
  - IDE integration (LSP)
  - Debugger

---

## Roadmap to Rust-Level Features

### Phase 1: Type System (Current Priority)

**Goal:** Achieve static type safety like Rust

#### Tasks:
1. **Strengthen Type Checker**
   - [ ] Enforce type annotations at compile time
   - [ ] Implement full type inference (Hindley-Milner style)
   - [ ] Add type error messages with suggestions
   - [ ] Create comprehensive type test suite

2. **Add Generics**
   - [ ] Design generic syntax: `chant identity<T>(x as T) as T`
   - [ ] Implement generic type checking
   - [ ] Support generic structs: `form Box<T> with value as T end`
   - [ ] Monomorphization in bytecode compiler
   - [ ] Test with common generic types (Result, Option equivalents)

3. **Trait System**
   - [ ] Design trait syntax (Rust-like interfaces)
   - [ ] Implement trait definitions
   - [ ] Implement trait implementations
   - [ ] Trait bounds on generics
   - [ ] Dynamic dispatch (trait objects)

**Success Criteria:**
- Generic functions work in all three execution engines
- Type errors caught at compile time (before execution)
- Zero runtime type errors in well-typed programs

### Phase 2: Ownership & Borrowing (Aspirational)

**Goal:** Prevent memory leaks and use-after-free without garbage collection

**Note:** This is ambitious and may require significant language redesign.

#### Tasks:
1. **Ownership Rules**
   - [ ] Design ownership semantics for Glimmer-Weave
   - [ ] Implement move semantics
   - [ ] Implement copy semantics
   - [ ] Borrow checker in semantic analyzer

2. **Lifetimes**
   - [ ] Design lifetime syntax
   - [ ] Lifetime inference
   - [ ] Lifetime annotations for complex cases

3. **Smart Pointers**
   - [ ] Reference-counted pointers (like Rc)
   - [ ] Atomic reference counting (like Arc)
   - [ ] Interior mutability (like RefCell)

**Success Criteria:**
- No manual memory management needed
- No garbage collection overhead
- Memory safety guaranteed at compile time

### Phase 3: Advanced Features

**Goal:** Match Rust's expressiveness and safety guarantees

#### Tasks:
1. **Macros / Metaprogramming**
   - [ ] Macro system design
   - [ ] Macro expansion in parser
   - [ ] Hygenic macros

2. **Async / Concurrency**
   - [ ] Async/await syntax
   - [ ] Future/Promise types
   - [ ] Async runtime

3. **FFI (Foreign Function Interface)**
   - [ ] Call C/Rust functions
   - [ ] Export Glimmer-Weave functions
   - [ ] ABI compatibility

4. **Standard Library Expansion**
   - [ ] Collections (Vec, HashMap, BTreeMap equivalents)
   - [ ] Iterators and iterator adapters
   - [ ] String manipulation
   - [ ] Filesystem I/O
   - [ ] Network I/O

**Success Criteria:**
- Can write production-grade applications
- Interoperates with existing Rust/C codebases
- Performance competitive with compiled languages

### Phase 4: Tooling & Ecosystem

**Goal:** Make Glimmer-Weave a productive language for real-world use

#### Tasks:
1. **Developer Tools**
   - [ ] REPL with syntax highlighting
   - [ ] Language Server Protocol (LSP) implementation
   - [ ] Formatter (like rustfmt)
   - [ ] Linter (like clippy)

2. **Build System**
   - [ ] Package manager (like Cargo)
   - [ ] Dependency resolution
   - [ ] Build scripts
   - [ ] Cross-compilation support

3. **Documentation**
   - [ ] Language reference manual
   - [ ] Tutorial series
   - [ ] API documentation for standard library
   - [ ] Example projects

**Success Criteria:**
- New users can get started in < 5 minutes
- IDE support for autocomplete, goto-definition, etc.
- Active community of contributors

---

## Common Tasks

### Adding a New Keyword

1. **Add token to `token.rs`:**

```rust
pub enum Token {
    // ... existing tokens

    /// `myfeature` - Description of feature
    MyFeature,
}

// In is_keyword()
| Token::MyFeature

// In is_statement_start() if it starts statements
| Token::MyFeature

// In description()
Token::MyFeature => "myfeature",
```

2. **Add lexer recognition in `lexer.rs`:**

```rust
// In keyword matching
"myfeature" => Token::MyFeature,
```

3. **Add AST node in `ast.rs`:**

```rust
pub enum AstNode {
    // ... existing variants

    MyFeature {
        param: String,
        body: Box<AstNode>,
    },
}
```

4. **Add parser in `parser.rs`:**

```rust
// In parse_statement()
Token::MyFeature => self.parse_my_feature(),

// Implementation
fn parse_my_feature(&mut self) -> ParseResult<AstNode> {
    self.expect(Token::MyFeature)?;

    let param = match self.current() {
        Token::Ident(n) => n.clone(),
        _ => return Err(ParseError {
            message: "Expected identifier".to_string(),
            position: self.position,
        }),
    };
    self.advance();

    self.expect(Token::Then)?;
    let body = self.parse_expression()?;
    self.expect(Token::End)?;

    Ok(AstNode::MyFeature {
        param,
        body: Box::new(body),
    })
}
```

5. **Add interpreter in `eval.rs`:**

```rust
// In eval_node()
AstNode::MyFeature { param, body } => {
    // Implement feature logic
    self.environment.define(param.clone(), Value::Number(42.0));
    self.eval_node(body)
}
```

6. **Add tests:**

```rust
#[test]
fn test_my_feature() {
    let source = r#"
        myfeature x then
            x + 10
        end
    "#;
    let result = eval_source(source).expect("Eval failed");
    assert_eq!(result, Value::Number(52.0));
}
```

7. **Add bytecode support (if needed):**

```rust
// In bytecode.rs - add instruction if needed
pub enum Instruction {
    // ... existing instructions
    MyFeatureOp { dest: Register, param: Register },
}

// In bytecode_compiler.rs
AstNode::MyFeature { param, body } => {
    // Compile to bytecode
    let param_reg = self.alloc_register()?;
    // ... emit instructions
    Ok(Some(param_reg))
}

// In vm.rs
Instruction::MyFeatureOp { dest, param } => {
    // Execute instruction
    self.registers[dest] = self.registers[param].clone();
}
```

8. **Add native codegen (or document limitation):**

```rust
// In codegen.rs
AstNode::MyFeature { param, body } => {
    // Either: generate x86-64 code
    self.emit(Instruction::Comment(format!("My feature: {}", param)));
    self.gen_expr(body)?;
    Ok(())

    // OR: document limitation
    Err("My feature requires heap allocation (not yet implemented in native codegen)".to_string())
}
```

### Adding a New Example Program

1. **Create `examples/NN_description.gw`:**

```glimmer-weave
# Glimmer-Weave My Feature Example
# Demonstrates the new feature

bind result to myfeature x then
    x * 2
end

result  # Should be 84 (if x = 42)
```

2. **Update `examples/README.md`:**

Add entry to the table:
```markdown
| `NN_description.gw` | Description of what it demonstrates | Expected output |
```

3. **Test manually:**

```bash
# Run in interpreter (when REPL exists)
glimmer-weave examples/NN_description.gw

# Or test via integration tests
cargo test test_example_NN_description
```

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_struct_definition

# Tests with output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration_test
```

### Debugging

```bash
# Check for compilation errors
cargo check

# Show all warnings
cargo build --all-features

# Run clippy for lint suggestions
cargo clippy

# Check formatting
cargo fmt --check
```

---

## Testing Strategy

### Test Organization

```
tests/
├── integration_test.rs    # End-to-end language tests
└── examples_test.rs       # Tests for example programs
```

### Test Categories

1. **Unit Tests** (in each module)
   - Lexer: Token recognition
   - Parser: AST generation
   - Interpreter: Value evaluation
   - Bytecode: Instruction emission
   - VM: Instruction execution

2. **Integration Tests**
   - Full programs end-to-end
   - Cross-module interactions
   - Example programs

3. **Property Tests** (future)
   - Fuzzing inputs
   - Randomized test generation
   - Invariant checking

### Writing Good Tests

```rust
#[test]
fn test_feature_name_what_it_tests() {
    // Arrange: Set up test data
    let source = r#"
        bind x to 42
        x + 10
    "#;

    // Act: Execute the code
    let result = eval_source(source);

    // Assert: Verify expectations
    assert!(result.is_ok(), "Evaluation should succeed");
    assert_eq!(result.unwrap(), Value::Number(52.0));
}

// Test error cases too!
#[test]
fn test_division_by_zero_error() {
    let source = r#"
        bind x to 10 / 0
    "#;

    let result = eval_source(source);
    assert!(result.is_err(), "Division by zero should fail");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("division by zero"));
}
```

**Golden Rules:**
- Test both success and failure cases
- Use descriptive test names
- Keep tests focused (one concept per test)
- Add comments for complex test logic
- Use `r#"..."#` raw strings for readability

---

## Emergency Contacts

### Critical Files - DO NOT DELETE

- `src/lib.rs` - Library entry point
- `src/token.rs` - Token definitions (breaking changes = parse errors everywhere)
- `src/ast.rs` - AST structure (breaking changes = compiler errors everywhere)
- `Cargo.toml` - Dependencies and metadata
- `.beads/` - Issue tracker database
- `examples/` - Example programs (used for testing)

### If Build Breaks

1. **Check Rust version:** `rustc --version` (should be stable)
2. **Clean build:** `cargo clean && cargo build`
3. **Check for typos in match statements:** Exhaustive matches will cause compiler errors if cases are missing
4. **Check test target:** Tests use `x86_64-pc-windows-msvc` (configured in `.cargo/config.toml`)
5. **Check dependencies:** `cargo update` (may help with version conflicts)

### Common Compiler Errors

**"non-exhaustive patterns"**
- You added an AST variant but didn't handle it everywhere
- Fix: Add match arms in `eval.rs`, `parser.rs`, `bytecode_compiler.rs`, etc.

**"borrow of moved value"**
- You moved a value but tried to use it again
- Fix: Clone the value (`value.clone()`) or use references

**"cannot find function/type in this scope"**
- Missing import or typo
- Fix: Add `use` statement or check spelling

---

## Philosophy Reminders

### When Stuck, Ask:

1. **"What would be natural to read?"** - Favor prose over symbols
2. **"How does Rust handle this?"** - Inherit Rust's safety principles
3. **"Can this fail?"** - If yes, use Result/Outcome, not panic
4. **"Is this feature in all three engines?"** - Maintain parity

### Naming Guidelines:

**Good Examples:**
- `bind` (immutable) vs. `weave` (mutable)
- `yield` (return value)
- `should...then...otherwise` (if/else)
- `attempt...harmonize` (try/catch)
- `Triumph` / `Mishap` (Ok/Err)

**Bad Examples:**
- `let`, `var`, `const` (too generic)
- `return` (imperative, not expressive)
- `if...else` (too terse)
- `try...catch` (exception-oriented)
- `Ok` / `Err` (too terse)

---

## Working with AI Assistants

### What to Preserve

- **Naming philosophy:** Natural language over terse syntax
- **Three-engine architecture:** Interpreter, bytecode VM, native codegen
- **Rust-inspired safety:** Result types, pattern matching, no null
- **Test coverage:** Every feature tested in all engines
- **bd issue tracking:** All work tracked via beads

### What Can Change

- **Implementation details:** Internal algorithms, data structures
- **Performance optimizations:** As long as correctness is maintained
- **Error messages:** Make them clearer and more helpful
- **Documentation:** Always improve clarity

### What Requires Discussion

- **Breaking syntax changes:** New keywords, operator precedence
- **Removing features:** Even if unused
- **Major architectural changes:** New execution engines, type system overhaul
- **Standard library design:** Function names, module organization

---

> **Remember:** Glimmer-Weave is both an experiment in natural language programming AND a production-quality scripting language for AethelOS. Prioritize clarity and safety over brevity and performance.

*Last updated: January 2025*
