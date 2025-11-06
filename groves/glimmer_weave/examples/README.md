# Glimmer-Weave Example Programs

This directory contains example programs written in Glimmer-Weave, the scripting language for AethelOS.

## Examples

| File | Description | Expected Output |
|------|-------------|-----------------|
| `01_hello_world.gw` | Basic string binding | "Hello, World!" |
| `02_arithmetic.gw` | Arithmetic operations (+, -, *, /, %) | 42 |
| `03_variables.gw` | Immutable (bind) and mutable (weave/set) variables | 15 |
| `04_conditionals.gw` | If/then/otherwise control flow | "Large number" |
| `05_loops.gw` | For-each iteration with ranges | 10 |
| `06_functions.gw` | Function definitions and calls | 30 |
| `07_lists.gw` | List literals and indexing | 20 |
| `08_maps.gw` | Map literals and field access | "Elara" |
| `09_factorial.gw` | Recursive factorial function | 120 |
| `10_fibonacci.gw` | Recursive Fibonacci function | 55 |
| `11_string_functions.gw` | String manipulation | Various |
| `12_math_functions.gw` | Mathematical operations | Various |
| `13_list_functions.gw` | List operations | Various |
| `14_map_functions.gw` | Map operations | Various |
| `15_type_conversion.gw` | Type conversions | Various |
| `16_comprehensive_test.gw` | Comprehensive language test | Various |
| `17_structs.gw` | Custom data types with named fields | 50 |

## Running Examples

Currently, Glimmer-Weave can be executed using the Rust interpreter:

```rust
use glimmer_weave::{Lexer, Parser, Evaluator};

// Read source file
let source = std::fs::read_to_string("examples/01_hello_world.gw")?;

// Tokenize
let mut lexer = Lexer::new(&source);
let tokens = lexer.tokenize();

// Parse
let mut parser = Parser::new(tokens);
let ast = parser.parse()?;

// Evaluate
let mut evaluator = Evaluator::new();
let result = evaluator.eval(&ast)?;

println!("{:?}", result);
```

## Language Features Demonstrated

### Variables
- **Immutable bindings**: `bind x to 42`
- **Mutable variables**: `weave counter as 0`
- **Mutation**: `set counter to 10`

### Control Flow
- **Conditionals**: `should x > 5 then ... otherwise ... end`
- **Loops**: `for each item in list then ... end`
- **Ranges**: `range(1, 10)`

### Functions
- **Definition**: `chant add(a, b) then ... end`
- **Return**: `yield result`
- **Recursion**: Fully supported
- **Closures**: Functions capture their environment

### Data Structures
- **Lists**: `[1, 2, 3]`
- **Maps**: `{name: "Elara", age: 42}`
- **Structs**: `form Person with name as Text age as Number end`
- **Indexing**: `list[0]`, `map[key]`
- **Field access**: `map.field`, `struct.field`

### Operators
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `>`, `<`, `>=`, `<=`, `is`, `is not`
- **Logical**: `and`, `or`, `not`

## Implementation Status

✅ **Complete**:
- Lexer (tokenization)
- Parser (AST generation)
- Evaluator (basic execution)
- Variables (bind, weave, set)
- Control flow (if, for, whilst)
- Functions (chant, yield)
- Data structures (lists, maps, structs)
- Arithmetic and logic
- Pattern matching (`match ... when`)
- Error handling (`attempt ... harmonize`)
- Outcome/Maybe types (`Triumph`, `Mishap`, `Present`, `Absent`)
- Custom structs (`form ... with ... end`)

⚪ **Not Yet Implemented**:
- Pipelines (`|`)
- World-Tree queries (`seek where`)
- Capability requests (`request ... with justification`)
- Bytecode compilation for structs
- Native codegen for structs

## Philosophy

Glimmer-Weave emphasizes natural expression over terse syntax:

```glimmer-weave
# Natural language keywords
should age >= 18 then
    VGA.write("Welcome, " + name)
otherwise
    VGA.write("Access denied")
end
```

The language is designed to feel like expressing intent rather than commanding a machine.
