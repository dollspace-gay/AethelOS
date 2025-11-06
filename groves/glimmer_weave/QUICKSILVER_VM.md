# Quicksilver VM (OS-114)

> A register-based bytecode virtual machine providing 3-6x performance improvement over tree-walking interpretation for Glimmer-Weave.

## Overview

The Quicksilver VM is a bytecode executor that compiles Glimmer-Weave AST to register-based bytecode for faster execution. It's designed as an optional fast path for pure expressions and computations.

**Status**: ✅ **Implemented and Benchmarked** (January 2025)

## Architecture

### Core Components

```
Source Code
    ↓
  Lexer → Tokens
    ↓
  Parser → AST
    ↓
  ┌─────────────────────┬─────────────────────┐
  │                     │                     │
  ↓                     ↓                     ↓
Tree-Walking        Bytecode              (Future: Native Code
Interpreter         Compiler               via Runic Forge)
  │                     │
  ↓                     ↓
Environment         Quicksilver VM
                        ↓
                    Result
```

### VM Design

**Type**: Register-based (like Lua 5.x, not stack-based like JVM)

**Register File**: 256 virtual registers (r0-r255)
- r0 is the return value register
- Registers are type-agnostic (hold any `Value`)
- Stack-based register allocator

**Instruction Set**: 40+ opcodes covering:
- Arithmetic: `AddNum`, `SubNum`, `MulNum`, `DivNum`, `ModNum`, `NegNum`
- Text operations: `ConcatText`
- Comparisons: `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`
- Logic: `Not`, `And`, `Or`
- Control flow: `Jump`, `JumpIfTrue`, `JumpIfFalse`
- Variables: `DefineGlobal`, `LoadGlobal`, `StoreGlobal`, `LoadLocal`, `StoreLocal`
- Data structures: `CreateList`, `CreateMap`, `GetIndex`, `SetIndex`, `GetField`, `SetField`
- Constants: `LoadConst`, `LoadNothing`, `LoadTruth`
- Flow control: `Halt`, `Return`, `Call`

**Constant Pool**: Deduplicates constants to reduce memory usage

**Features**:
- Type-aware instructions (separate opcodes for numbers, text, truth)
- Jump patching for forward jumps in control flow
- Efficient register allocation/deallocation

## Performance

### Benchmark Results (Release Mode)

| Benchmark | Interpreter | VM | Speedup |
|-----------|-------------|-----|---------|
| Simple Arithmetic (`10 + 20 * 2`) | 12.32 µs | 3.22 µs | **3.83x** |
| Complex Expression (`((10 + 20) * 3 - 5) / 2 + 100`) | 14.07 µs | 2.41 µs | **5.84x** |
| Global Variables (`bind x to 42; x + 8`) | 11.36 µs | 3.48 µs | **3.26x** |
| Comparisons (`10 < 20 and 30 > 15 or 5 == 5`) | 11.48 µs | 2.36 µs | **4.87x** |
| Fibonacci-like (10 binds + arithmetic) | 11.23 µs | 8.78 µs | **1.28x** |

**Average Speedup**: ~3.8x faster (up to 5.84x for complex expressions)

### When to Use the VM

**✅ Good for**:
- Pure arithmetic expressions
- Comparisons and logic
- Global variable access
- Simple data structures (lists, maps)
- Hot paths in scripts (loops with pure computations)

**❌ Not ideal for**:
- Code with many variable bindings (compilation overhead)
- Function-heavy code (chants not yet optimized)
- I/O-bound operations
- Code that runs once (compilation cost not amortized)

## Usage

### Basic Usage

```rust
use glimmer_weave::{Lexer, Parser, Evaluator};

let source = "10 + 20 * 2";

// Parse
let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();
let mut parser = Parser::new(tokens);
let ast = parser.parse()?;

// Option 1: Tree-walking interpreter (default)
let mut evaluator = Evaluator::new();
let result = evaluator.eval(&ast)?; // Returns Value::Number(50.0)

// Option 2: Bytecode VM (faster for repeated execution)
let result = evaluator.eval_with_vm(&ast)?; // Returns Value::Number(50.0)
```

### Advanced: Compile Once, Execute Many Times

```rust
use glimmer_weave::bytecode_compiler::compile;
use glimmer_weave::vm::VM;

// Parse and compile once
let chunk = compile(&ast)?;

// Execute many times (amortizes compilation cost)
for _ in 0..10000 {
    let mut vm = VM::new();
    let result = vm.execute(chunk.clone())?;
    // Process result...
}
```

## Implementation Details

### Module Structure

- **[bytecode.rs](src/bytecode.rs)** (~530 lines)
  - `Instruction` enum with 40+ opcodes
  - `BytecodeChunk` with constant pooling
  - `Disassembler` for debugging

- **[bytecode_compiler.rs](src/bytecode_compiler.rs)** (~470 lines)
  - AST → Bytecode transformation
  - Register allocation
  - Jump patching
  - Variable scoping (local/global)

- **[vm.rs](src/vm.rs)** (~420 lines)
  - Register file (256 registers)
  - Fetch-decode-execute loop
  - Type-safe operations
  - Global variable storage

- **[eval.rs](src/eval.rs)** (integration)
  - `eval_with_vm()` method
  - Error mapping (VmError → RuntimeError)

### Example Bytecode

Source: `10 + 20 * 2`

```
LoadConst r0, #0     ; r0 = 10
LoadConst r1, #1     ; r1 = 20
LoadConst r2, #2     ; r2 = 2
MulNum r3, r1, r2    ; r3 = r1 * r2 = 40
AddNum r2, r0, r3    ; r2 = r0 + r3 = 50
Move r0, r2          ; r0 = r2 (return value)
Halt                 ; return r0
```

## Testing

### Running Tests

```bash
# Unit tests (bytecode, compiler, VM)
cargo test --lib

# Integration tests (VM vs interpreter equivalence)
cargo test eval::tests::test_vm_integration

# Benchmarks (debug mode)
cargo test --test benchmark run_all_benchmarks -- --nocapture --ignored

# Benchmarks (release mode, shows real performance)
cargo test --release --test benchmark run_all_benchmarks -- --nocapture --ignored
```

### Test Coverage

- ✅ **5/5** bytecode module tests (constant pooling, jump patching, disassembly)
- ✅ **5/5** compiler tests (arithmetic, control flow, variables)
- ✅ **4/4** VM execution tests (numbers, arithmetic, comparisons, variables)
- ✅ **3/3** integration tests (VM produces same results as interpreter)
- ✅ **5/5** benchmarks (performance validation)

**Total: 22/22 tests passing**

## Limitations & Future Work

### Current Limitations

1. **Local variables**: Only global variables are fully optimized
   - Local variables use basic stack-based allocation
   - Not yet optimized for nested scopes

2. **Function calls**: Not yet optimized
   - Chants (functions) still use closure-based execution
   - No bytecode-level function call optimization

3. **Capability requests**: Not supported in VM
   - Must fall back to interpreter for kernel interactions

4. **Advanced control flow**: Limited support
   - `for` loops, pattern matching not yet compiled
   - `attempt`/`harmonize` error handling not yet supported

### Future Enhancements

See related BD issues:

- **OS-113**: Runic Forge (AOT compilation to native code) - 10-100x speedup
- **OS-115**: Memory efficiency (borrowing/ownership semantics)
- **OS-116**: Concurrency model (Harmonic threads)
- **OS-118**: Zero-cost abstractions

### Optimization Opportunities

1. **Instruction fusion**: Combine common patterns (e.g., LoadConst + AddNum → AddConstNum)
2. **Register reuse**: More aggressive register recycling
3. **Inline caching**: Cache variable lookups for hot paths
4. **Trace compilation**: JIT compile hot loops
5. **SIMD**: Vectorize arithmetic for arrays/lists

## Design Philosophy

The Quicksilver VM follows AethelOS design principles:

- **Harmony Over Force**: Cooperative compilation, not mandatory
  - Tree-walking interpreter remains the default
  - VM is an *optional* fast path

- **Memory Over Forgetting**: Bytecode can be cached
  - Compile once, execute many times
  - Future: Persistent bytecode cache for hot scripts

- **Beauty as Necessity**: Clean bytecode design
  - Register-based (simpler mental model than stack-based)
  - Type-aware instructions (readable disassembly)
  - Minimal opcodes (no bloat)

## References

### Inspiration

- **Lua 5.x**: Register-based VM design
- **CPython**: Bytecode compilation strategy
- **V8/SpiderMonkey**: JIT compilation techniques (future work)

### Related Documentation

- [WEAVE_MARKS.md](WEAVE_MARKS.md) - Type annotation system (OS-112)
- [OS-113](../.beads/issues.jsonl) - Native code compilation (Runic Forge)
- [OS-114](../.beads/issues.jsonl) - This implementation
- [OS-115](../.beads/issues.jsonl) - Memory efficiency improvements

---

**Author**: Claude (with human oversight)
**Date**: January 2025
**License**: Same as AethelOS (see root LICENSE)
