# Weave-Marks: Glimmer-Weave Type System

> **Status:** Implemented (OS-112) ✓
> **Date:** January 2025
> **Type System:** Optional Static Typing with Gradual Typing

---

## Overview

Weave-Marks is Glimmer-Weave's optional static type system. It allows developers to add type annotations to their code for improved safety and documentation, while maintaining full backward compatibility with untyped code.

### Philosophy

- **Optional**: Types are never required, they're hints for the compiler and documentation
- **Gradual**: Mix typed and untyped code freely in the same program
- **Natural**: Type syntax reads like natural language, matching Glimmer-Weave's aesthetic
- **Progressive**: Start with dynamic typing, add types incrementally as code matures

---

## Basic Syntax

### Type Annotations

Types are annotated using a colon `:` followed by the type name:

```glimmer-weave
bind x: Number to 42
bind name: Text to "Alice"
bind flag: Truth to true
bind void: Nothing to nothing
```

### Mutable Variables

Mutable variables support type annotations:

```glimmer-weave
weave counter: Number as 0
set counter to counter + 1
```

### Function Parameters

Function parameters can have optional type annotations:

```glimmer-weave
chant greet(name: Text) then
    yield "Hello, " + name
end

chant add(a: Number, b: Number) then
    yield a + b
end
```

### Return Types

Functions can declare their return type using `->`:

```glimmer-weave
chant factorial(n: Number) -> Number then
    should n <= 1 then
        yield 1
    otherwise
        yield n * factorial(n - 1)
    end
end
```

---

## Built-in Types

### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `Number` | Floating-point numbers | `42`, `3.14`, `-7` |
| `Text` | String values | `"hello"`, `"world"` |
| `Truth` | Boolean values | `true`, `false` |
| `Nothing` | Null/void value | `nothing` |

### Collection Types

| Type | Description | Example |
|------|-------------|---------|
| `List<T>` | Generic list with element type | `[1, 2, 3]` |
| `Map` | Key-value map | `{name: "Alice", age: 42}` |

### Function Types

Functions have implicit types based on their signatures:

```glimmer-weave
Function<(Number, Number) -> Number>
Function<(Text) -> Text>
Function<() -> Nothing>
```

---

## Gradual Typing

### Mixing Typed and Untyped Code

You can freely mix typed and untyped code in the same program:

```glimmer-weave
# Untyped variable
bind x to 10

# Typed variable
bind y: Number to 20

# Untyped function
chant double(n) then
    yield n * 2
end

# Typed function
chant triple(n: Number) -> Number then
    yield n * 3
end

# Both work together
double(x)  # OK
triple(y)  # OK
double(y)  # OK - typed value to untyped function
triple(x)  # OK - untyped value to typed function
```

### Partial Typing

You can type some parameters but not others:

```glimmer-weave
chant process(x: Number, y, z: Text) then
    # x is typed as Number
    # y is untyped (Any)
    # z is typed as Text
    yield x + y
end
```

### Return Type Inference

Return types can be omitted; they'll be inferred as `Any`:

```glimmer-weave
# Explicit return type
chant add(a: Number, b: Number) -> Number then
    yield a + b
end

# Inferred return type (also valid)
chant add(a: Number, b: Number) then
    yield a + b
end
```

---

## Type Checking

### Semantic Analysis

The semantic analyzer validates type compatibility when annotations are provided:

```glimmer-weave
# ✓ Valid: type matches
bind x: Number to 42

# ✗ Error: type mismatch
bind x: Number to "not a number"
```

### Runtime Behavior

**Important**: Type annotations are checked at compile-time but **ignored at runtime**. Glimmer-Weave uses gradual typing, so:

- Typed code runs at the same speed as untyped code
- No runtime type checks or overhead
- Type errors are caught during semantic analysis
- Runtime behavior is purely dynamic

```glimmer-weave
# Semantic analyzer catches this at compile-time:
bind x: Number to "text"  # TypeError during analysis

# But evaluation proceeds with dynamic typing
bind y to "text"  # OK - no type annotation
```

---

## Operator Type Rules

### Addition (`+`)

The `+` operator is **polymorphic** and supports:

1. **Numeric Addition**: `Number + Number → Number`
2. **String Concatenation**: `Text + Text → Text`

```glimmer-weave
bind sum: Number to 10 + 20        # → 30
bind greeting: Text to "Hello" + " " + "World"  # → "Hello World"

# Type error: mixed types
bind bad to 10 + "text"  # ✗ TypeError
```

### Other Arithmetic Operators

All other arithmetic operators require `Number` operands:

```glimmer-weave
bind x: Number to 10 - 5   # Subtraction
bind y: Number to 10 * 5   # Multiplication
bind z: Number to 10 / 5   # Division
bind r: Number to 10 % 3   # Modulo

# These all require Number types
bind bad to "text" - "other"  # ✗ TypeError
```

### Comparison Operators

Comparison operators work with any type and return `Truth`:

```glimmer-weave
bind isEqual: Truth to (10 is 10)
bind isGreater: Truth to (20 > 10)
bind isLess: Truth to (5 < 10)
```

### Logical Operators

Logical operators work with any truthy/falsy value and return `Truth`:

```glimmer-weave
bind result: Truth to true and false
bind result: Truth to true or false
bind result: Truth to not false
```

---

## Examples

### Example 1: Basic Typed Function

```glimmer-weave
chant celsius_to_fahrenheit(celsius: Number) -> Number then
    yield celsius * 9 / 5 + 32
end

bind temp: Number to celsius_to_fahrenheit(25)
# temp = 77
```

### Example 2: Typed Recursive Function

```glimmer-weave
chant fibonacci(n: Number) -> Number then
    should n <= 1 then
        yield n
    otherwise
        yield fibonacci(n - 1) + fibonacci(n - 2)
    end
end

bind fib10: Number to fibonacci(10)
# fib10 = 55
```

### Example 3: Typed Loop with Mutable Variables

```glimmer-weave
chant sum_range(start: Number, end: Number) -> Number then
    weave total: Number as 0
    weave i: Number as start

    whilst i <= end then
        set total to total + i
        set i to i + 1
    end

    yield total
end

bind result: Number to sum_range(1, 10)
# result = 55
```

### Example 4: String Processing

```glimmer-weave
chant greet(name: Text, title: Text) -> Text then
    yield "Hello, " + title + " " + name + "!"
end

bind message: Text to greet("Alice", "Dr.")
# message = "Hello, Dr. Alice!"
```

### Example 5: List Processing

```glimmer-weave
bind numbers: List<Number> to [1, 2, 3, 4, 5]
bind first: Number to numbers[0]
bind third: Number to numbers[2]
```

### Example 6: Gradual Migration

Start with untyped code:

```glimmer-weave
chant calculate(x, y, z) then
    yield x * y + z
end
```

Add types incrementally:

```glimmer-weave
chant calculate(x: Number, y: Number, z: Number) then
    yield x * y + z
end
```

Add return type:

```glimmer-weave
chant calculate(x: Number, y: Number, z: Number) -> Number then
    yield x * y + z
end
```

---

## Implementation Details

### AST Representation

Type annotations are represented in the AST as `TypeAnnotation` enums:

```rust
pub enum TypeAnnotation {
    Named(String),              // Number, Text, Truth, Nothing
    List(Box<TypeAnnotation>),  // List<T>
    Map,                        // Map
    Function {
        param_types: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },
    Optional(Box<TypeAnnotation>),  // Future: Number?
}
```

### Semantic Type System

The semantic analyzer converts AST type annotations to internal types:

```rust
pub enum Type {
    Number,
    Text,
    Truth,
    Nothing,
    List(Box<Type>),
    Map,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Any,      // Untyped values
    Unknown,  // Type errors
}
```

### Type Checking Algorithm

1. **Parse Phase**: Parser recognizes type annotations and builds AST
2. **Semantic Phase**: Analyzer converts annotations to semantic types
3. **Validation Phase**: Check type compatibility at assignment points
4. **Error Reporting**: Collect all type errors with context
5. **Evaluation Phase**: Ignore type annotations, run dynamically

### Compatibility Rules

Types are compatible if:

- They are exactly equal (`Number` with `Number`)
- One is `Any` (untyped)
- One is `Unknown` (type error already reported)

Special case for `Add` operator:
- `Number + Number → Number`
- `Text + Text → Text`
- Mixed types are incompatible

---

## Testing

### Running Type Tests

```bash
# Run all type annotation tests
cd /f/OS/groves/glimmer_weave
cargo test --test type_annotations_test

# Run specific test
cargo test test_typed_function_with_recursion
```

### Test Coverage

The [type_annotations_test.rs](tests/type_annotations_test.rs) file includes:

- ✓ Typed bind statements
- ✓ Typed weave statements
- ✓ Typed function parameters
- ✓ Typed return types
- ✓ Recursive typed functions
- ✓ Mixed typed/untyped code
- ✓ Semantic analysis validation
- ✓ Type error detection
- ✓ List type annotations
- ✓ Partial parameter typing
- ✓ String concatenation with types

---

## Future Enhancements

### Planned Features (Not Yet Implemented)

1. **Optional Types**: `Number?` for nullable values
2. **Union Types**: `Number | Text` for multiple possible types
3. **Type Aliases**: `type UserId as Number`
4. **Generic Functions**: `chant<T> identity(x: T) -> T`
5. **Trait Bounds**: `chant<T: Numeric> add(a: T, b: T) -> T`
6. **Struct Types**: Custom data structures with typed fields
7. **Type Inference**: Infer types from usage patterns
8. **Refined Types**: `Number(0..100)` for range constraints

### Related Issues

- **OS-113**: Native code compilation (Runic Forge AOT)
- **OS-114**: Memory safety guarantees
- **OS-115**: Ownership and borrowing semantics
- **OS-116**: Concurrency safety (Send/Sync traits)
- **OS-117**: Error handling with Result types
- **OS-118**: Zero-cost abstractions

---

## Design Decisions

### Why Gradual Typing?

1. **Backwards Compatibility**: Existing code continues to work
2. **Progressive Enhancement**: Add types where they help most
3. **Rapid Prototyping**: Start dynamic, formalize later
4. **Learning Curve**: Beginners can ignore types initially
5. **AethelOS Philosophy**: Harmony over force - types guide, don't constrain

### Why No Runtime Overhead?

1. **Performance**: Typed code runs as fast as untyped
2. **Simplicity**: No runtime type system to maintain
3. **Predictability**: Behavior is purely dynamic, easy to reason about
4. **Compatibility**: Seamless interop between typed and untyped code

### Why Natural Syntax?

Glimmer-Weave prioritizes readability:

```glimmer-weave
# Reads like English
bind x: Number to 42
chant add(a: Number, b: Number) -> Number

# Not like traditional languages
let x: i32 = 42;
fn add(a: i32, b: i32) -> i32
```

---

## References

### Related Files

- **AST**: [src/ast.rs](src/ast.rs) - Type annotation AST nodes
- **Lexer**: [src/lexer.rs](src/lexer.rs) - Arrow token for return types
- **Parser**: [src/parser.rs](src/parser.rs) - Type annotation parsing
- **Semantic**: [src/semantic.rs](src/semantic.rs) - Type checking logic
- **Evaluator**: [src/eval.rs](src/eval.rs) - Runtime behavior (ignores types)
- **Tests**: [tests/type_annotations_test.rs](tests/type_annotations_test.rs)

### Documentation

- **Main README**: [README.md](README.md) - Project overview
- **Type System**: This file (WEAVE_MARKS.md)

### Issue Tracking

- **BD Issue**: OS-112 (Closed) - Optional static typing implementation
- **Related**: OS-113 through OS-118 - Additional Rust strength inheritance

---

## Troubleshooting

### Build Configuration for Tests

**Issue**: Tests fail with duplicate lang item errors due to workspace `build-std` configuration.

**Solution**: When running tests, temporarily comment out the `[unstable]` section in `F:\OS\.cargo\config.toml`:

```toml
# Temporarily comment for glimmer_weave tests
#[unstable]
#build-std = ["core", "compiler_builtins", "alloc"]
#build-std-features = ["compiler-builtins-mem"]
```

Then run tests normally:

```bash
cd /f/OS/groves/glimmer_weave
cargo test
```

**Remember** to restore the configuration after testing for kernel builds.

### Common Type Errors

**Error**: `TypeError: expected Number, got Text`

```glimmer-weave
bind x: Number to "not a number"  # ✗
```

**Solution**: Ensure the value matches the declared type:

```glimmer-weave
bind x: Number to 42  # ✓
```

---

**Error**: `TypeError: addition/concatenation requires matching types`

```glimmer-weave
bind mixed to 10 + "text"  # ✗
```

**Solution**: Use consistent types for `+` operator:

```glimmer-weave
bind sum to 10 + 20        # ✓ Number + Number
bind text to "a" + "b"     # ✓ Text + Text
```

---

## Contributing

When adding new type system features:

1. Update `TypeAnnotation` enum in [ast.rs](src/ast.rs)
2. Add lexer support if new tokens needed in [lexer.rs](src/lexer.rs)
3. Update parser in [parser.rs](src/parser.rs)
4. Add semantic type in [semantic.rs](src/semantic.rs)
5. Implement type checking logic
6. Update evaluator if runtime behavior changes in [eval.rs](src/eval.rs)
7. Add comprehensive tests in [tests/type_annotations_test.rs](tests/type_annotations_test.rs)
8. Update this documentation

---

**Last Updated**: January 2025
**Status**: Stable (OS-112 Complete) ✓
**Maintainer**: AethelOS Team
