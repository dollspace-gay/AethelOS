# The Glimmer-Forge Plan
## Scripting Language & Compiler Architecture for AethelOS

> *"The word shapes the world. The rune shapes eternity."*

---

## Document Overview

This document describes the architecture and implementation plan for AethelOS's dual language system:

1. **Glimmer-Weave** (Priority 1) - A scripting language for everyday tasks, shell interaction, and rapid prototyping
2. **The Runic Forge** (Priority 2) - A compiler service for creating permanent kernel extensions and high-performance applications

Together, these systems enable AethelOS to evolve and grow while maintaining the harmony and security principles of symbiotic computing.

---

# Part I: Glimmer-Weave Scripting Language

## Philosophy

Glimmer-Weave is the living voice of AethelOS. It allows users to:
- Interact with the system conversationally through the Eldarin shell
- Automate tasks with scripts
- Query and manipulate the World-Tree filesystem
- Prototype ideas before forging them into permanent runes

Like water flowing through stone, Glimmer-Weave scripts are ephemeral but powerful. They exist to serve the moment, not eternity.

## Language Design Principles

1. **Natural Expression** - Syntax that reads like intention, not machinery
2. **Query-First** - Native support for World-Tree queries
3. **Capability-Aware** - Security built into the language
4. **Harmonic Failure** - Errors are suggestions, not crashes
5. **Contextual Flow** - State flows naturally through pipelines

## Syntax Overview

### Core Syntax Elements

```glimmer-weave
# Variable binding (immutable by default)
bind scroll_title to "The Lay of Lúthien"
bind age to 42
bind colors to ["cyan", "magenta", "yellow"]

# Mutable binding
weave counter as 0           # 'weave' = mutable
set counter to 10            # mutation

# Conditionals (natural language)
should sky is blue then
    VGA.set_color("cyan")
end

should not file_exists then
    VGA.write("File not found")
otherwise
    VGA.write("File exists!")
end

# Loops
for each star in celestial_map then
    star.twinkle()
end

for count in range(1, 10) then
    VGA.write(count)
end

# Functions (called "chants")
chant greet(name) then
    VGA.write("Mae govannen, " + name)
end

chant calculate(x, y) then
    yield x + y              # 'yield' = return
end

# Calling chants
greet("Elara")
bind result to calculate(5, 7)

# World-Tree queries (native syntax)
seek scrolls where essence is "Scroll" and creator is "Elara"

bind my_scrolls to seek where
    essence is "Scroll" and
    creator is "Elara" and
    created after 2025-01-01
end

# Pipelines (data flow)
seek where essence is "Scroll"
    | filter by creator is "Elara"
    | sort by created descending
    | take 10
    | for each scroll then VGA.write(scroll.name) end

# Capability requests
request VGA.write with justification "Display welcome message"
request FileSystem.create with justification "Save user preferences"

# Pattern matching
match file_type with
    when "Scroll" then handle_text_file()
    when "Rune" then handle_binary()
    when "Glyph" then handle_image()
    otherwise handle_unknown()
end

# Error handling (harmonic failure)
attempt
    bind scroll to seek where name is "config.txt" | first
    VGA.write(scroll.content)
harmonize on NotFound then
    VGA.write("Config file not found, using defaults")
harmonize on PermissionDenied then
    VGA.write("Cannot read config file")
end
```

### Type System

Glimmer-Weave is **dynamically typed** but **capability-enforced**:

**Primitive Types:**
- `Number` - 64-bit signed integer or float (auto-detected)
- `Text` - UTF-8 string
- `Truth` - Boolean (`true`, `false`)
- `Nothing` - Null/void value

**Collection Types:**
- `List` - Ordered sequence: `[1, 2, 3]`
- `Map` - Key-value pairs: `{name: "Elara", age: 42}`

**System Types:**
- `Scroll` - File handle from World-Tree
- `Capability` - Unforgeable permission token
- `Thread` - Thread handle
- `Moment` - Timestamp

**Example:**
```glimmer-weave
bind numbers to [1, 2, 3, 4, 5]
bind person to {name: "Elara", age: 142, role: "Weaver"}
bind scroll to seek where name is "poem.txt" | first
```

### Operators

**Arithmetic:** `+`, `-`, `*`, `/`, `%`
**Comparison:** `is`, `is not`, `>`, `<`, `>=`, `<=`
**Logical:** `and`, `or`, `not`
**Pipeline:** `|` (passes data through transformations)

**Examples:**
```glimmer-weave
bind sum to 5 + 3
bind equal to name is "Elara"
bind adult to age >= 18 and age < 100
bind result to numbers | filter by x > 5 | sum
```

## Kernel API Bindings

Glimmer-Weave scripts interact with AethelOS through **capability-protected kernel APIs**. These APIs are exposed as namespaced objects.

### VGA Module (Display)

```glimmer-weave
# Write text to screen
VGA.write("Hello, AethelOS!")
VGA.writeln("With newline")

# Set colors
VGA.set_color(foreground: "cyan", background: "black")
VGA.set_fg("white")
VGA.set_bg("blue")

# Clear screen
VGA.clear()

# Cursor control
VGA.move_cursor(row: 5, col: 10)
VGA.hide_cursor()
VGA.show_cursor()
```

### WorldTree Module (Filesystem)

```glimmer-weave
# Seek files (returns list of Scrolls)
bind scrolls to WorldTree.seek(
    essence: "Scroll",
    creator: "Elara"
)

# Create new scroll
bind scroll to WorldTree.create(
    name: "my-poem.txt",
    essence: "Scroll",
    content: "In the beginning..."
)

# Read content
bind content to WorldTree.read(scroll)

# Update (creates new version)
WorldTree.update(scroll, content: "New content here")

# Get metadata
bind meta to WorldTree.metadata(scroll)
VGA.write("Created: " + meta.created)

# Time travel (read old version)
bind old to WorldTree.read_at(scroll, timestamp: yesterday)

# Tag management
WorldTree.add_tag(scroll, key: "project", value: "AethelOS")
bind tagged to WorldTree.seek(tag: "project=AethelOS")
```

### Loom Module (Threading)

```glimmer-weave
# Spawn a new thread
bind thread to Loom.spawn(
    priority: "Normal",
    chant: worker_function
)

# Yield to other threads
Loom.yield()

# Sleep
Loom.sleep(milliseconds: 100)

# Get current thread info
bind info to Loom.current()
VGA.write("Thread ID: " + info.id)
```

### Nexus Module (Message Passing - Future)

```glimmer-weave
# Send message to thread
Nexus.send(thread, message: {type: "greeting", text: "Hello"})

# Receive message (blocks)
bind msg to Nexus.receive()

# Receive with timeout
bind msg to Nexus.receive(timeout: 1000)
should msg is Nothing then
    VGA.write("No message received")
end
```

### Time Module

```glimmer-weave
# Current timestamp
bind now to Time.now()

# Create moments
bind yesterday to Time.now() - Time.days(1)
bind next_week to Time.now() + Time.weeks(1)

# Format time
VGA.write(Time.format(now, "YYYY-MM-DD HH:mm:ss"))
```

### Keyboard Module

```glimmer-weave
# Read single key (blocks)
bind key to Keyboard.read_key()

# Read line (blocks until Enter)
bind line to Keyboard.read_line()

# Check if key is available (non-blocking)
should Keyboard.has_key() then
    bind key to Keyboard.read_key()
end
```

## Interpreter Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────┐
│                Glimmer-Weave Script                 │
│          "bind x to 5; VGA.write(x)"               │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│                  Lexer/Tokenizer                    │
│  Converts text → tokens (BIND, IDENT, NUMBER, etc) │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│                      Parser                         │
│    Converts tokens → Abstract Syntax Tree (AST)    │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│                    Evaluator                        │
│   Walks AST, executes operations, manages state    │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│              Kernel API Dispatcher                  │
│   Routes API calls → kernel services (VGA, etc)    │
└─────────────────────────────────────────────────────┘
```

### Core Data Structures

#### Token
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Bind, Weave, Set, To, As,
    Should, Then, End, Otherwise,
    For, Each, In,
    Chant, Yield,
    Seek, Where,
    Attempt, Harmonize, On,
    Match, When,

    // Literals
    Number(f64),
    Text(String),
    Truth(bool),
    Nothing,

    // Identifiers
    Ident(String),

    // Operators
    Plus, Minus, Star, Slash, Percent,
    Is, IsNot, Greater, Less, GreaterEq, LessEq,
    And, Or, Not,
    Pipe,

    // Delimiters
    LeftParen, RightParen,
    LeftBracket, RightBracket,
    LeftBrace, RightBrace,
    Comma, Colon, Dot,

    // Special
    Newline,
    Eof,
}
```

#### AST Node
```rust
#[derive(Debug, Clone)]
pub enum AstNode {
    // Statements
    BindStmt { name: String, value: Box<AstNode> },
    WeaveStmt { name: String, value: Box<AstNode> },
    SetStmt { name: String, value: Box<AstNode> },

    IfStmt {
        condition: Box<AstNode>,
        then_branch: Vec<AstNode>,
        else_branch: Option<Vec<AstNode>>,
    },

    ForStmt {
        variable: String,
        iterable: Box<AstNode>,
        body: Vec<AstNode>,
    },

    ChantDef {
        name: String,
        params: Vec<String>,
        body: Vec<AstNode>,
    },

    YieldStmt { value: Box<AstNode> },

    SeekExpr {
        conditions: Vec<(String, AstNode)>, // field, value pairs
    },

    // Expressions
    Number(f64),
    Text(String),
    Truth(bool),
    Nothing,
    Ident(String),

    List(Vec<AstNode>),
    Map(Vec<(String, AstNode)>),

    BinaryOp {
        left: Box<AstNode>,
        op: BinaryOperator,
        right: Box<AstNode>,
    },

    UnaryOp {
        op: UnaryOperator,
        operand: Box<AstNode>,
    },

    Call {
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },

    FieldAccess {
        object: Box<AstNode>,
        field: String,
    },

    Pipeline {
        stages: Vec<AstNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div, Mod,
    Equal, NotEqual, Greater, Less, GreaterEq, LessEq,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not, Negate,
}
```

#### Runtime Value
```rust
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Truth(bool),
    Nothing,

    List(Vec<Value>),
    Map(HashMap<String, Value>),

    Scroll(ScrollHandle),       // File from World-Tree
    Capability(CapabilityToken),
    Thread(ThreadId),
    Moment(Timestamp),

    Chant {
        params: Vec<String>,
        body: Vec<AstNode>,
        closure: Environment,   // Captured variables
    },

    NativeFunction {
        name: String,
        func: fn(&[Value]) -> Result<Value, RuntimeError>,
    },
}
```

### Lexer Implementation

The lexer converts source text into tokens:

```rust
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.get(0).copied();
        Lexer {
            input: chars,
            position: 0,
            current_char,
        }
    }

    pub fn next_token(&mut self) -> Token {
        // Skip whitespace (except newlines)
        self.skip_whitespace();

        // Match current character
        match self.current_char {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('#') => {
                self.skip_comment();
                self.next_token()
            }
            Some('"') => self.read_string(),
            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() => self.read_identifier_or_keyword(),
            Some('+') => { self.advance(); Token::Plus }
            Some('-') => { self.advance(); Token::Minus }
            Some('*') => { self.advance(); Token::Star }
            Some('/') => { self.advance(); Token::Slash }
            // ... more operators ...
            _ => panic!("Unexpected character: {}", self.current_char.unwrap()),
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.position;
        while let Some(c) = self.current_char {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.position].iter().collect();

        // Check if it's a keyword
        match text.as_str() {
            "bind" => Token::Bind,
            "weave" => Token::Weave,
            "set" => Token::Set,
            "to" => Token::To,
            "should" => Token::Should,
            "then" => Token::Then,
            "end" => Token::End,
            "true" => Token::Truth(true),
            "false" => Token::Truth(false),
            "nothing" => Token::Nothing,
            // ... more keywords ...
            _ => Token::Ident(text),
        }
    }
}
```

### Parser Implementation

The parser builds an AST from tokens:

```rust
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<AstNode>, ParseError> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip newlines
            while self.match_token(&Token::Newline) {
                self.advance();
            }

            if self.is_at_end() {
                break;
            }

            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<AstNode, ParseError> {
        match &self.current() {
            Token::Bind => self.parse_bind_statement(),
            Token::Weave => self.parse_weave_statement(),
            Token::Set => self.parse_set_statement(),
            Token::Should => self.parse_if_statement(),
            Token::For => self.parse_for_statement(),
            Token::Chant => self.parse_chant_definition(),
            Token::Yield => self.parse_yield_statement(),
            Token::Seek => self.parse_seek_expression(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_bind_statement(&mut self) -> Result<AstNode, ParseError> {
        self.expect(Token::Bind)?;

        let name = match self.current() {
            Token::Ident(s) => s.clone(),
            _ => return Err(ParseError::ExpectedIdentifier),
        };
        self.advance();

        self.expect(Token::To)?;

        let value = self.parse_expression()?;

        Ok(AstNode::BindStmt {
            name,
            value: Box::new(value),
        })
    }

    fn parse_expression(&mut self) -> Result<AstNode, ParseError> {
        self.parse_pipeline()
    }

    fn parse_pipeline(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_logical_or()?;

        if self.match_token(&Token::Pipe) {
            let mut stages = vec![left];

            while self.match_token(&Token::Pipe) {
                self.advance();
                stages.push(self.parse_logical_or()?);
            }

            return Ok(AstNode::Pipeline { stages });
        }

        Ok(left)
    }

    // ... more parsing methods (parse_logical_or, parse_logical_and,
    //     parse_equality, parse_comparison, parse_term, parse_factor,
    //     parse_unary, parse_call, parse_primary) ...
}
```

### Evaluator Implementation

The evaluator executes the AST:

```rust
pub struct Evaluator {
    environment: Environment,
    kernel_apis: KernelApis,
}

pub struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.scopes.last_mut().unwrap().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        None
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

impl Evaluator {
    pub fn new(kernel_apis: KernelApis) -> Self {
        let mut env = Environment::new();

        // Register built-in functions
        env.define("range".to_string(), Value::NativeFunction {
            name: "range".to_string(),
            func: builtin_range,
        });

        Evaluator {
            environment: env,
            kernel_apis,
        }
    }

    pub fn eval(&mut self, nodes: Vec<AstNode>) -> Result<Value, RuntimeError> {
        let mut last_value = Value::Nothing;

        for node in nodes {
            last_value = self.eval_node(&node)?;
        }

        Ok(last_value)
    }

    fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
        match node {
            AstNode::Number(n) => Ok(Value::Number(*n)),
            AstNode::Text(s) => Ok(Value::Text(s.clone())),
            AstNode::Truth(b) => Ok(Value::Truth(*b)),
            AstNode::Nothing => Ok(Value::Nothing),

            AstNode::Ident(name) => {
                self.environment.get(name)
                    .cloned()
                    .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))
            }

            AstNode::BindStmt { name, value } => {
                let val = self.eval_node(value)?;
                self.environment.define(name.clone(), val.clone());
                Ok(val)
            }

            AstNode::SetStmt { name, value } => {
                let val = self.eval_node(value)?;
                self.environment.set(name, val.clone())?;
                Ok(val)
            }

            AstNode::IfStmt { condition, then_branch, else_branch } => {
                let cond_value = self.eval_node(condition)?;

                if self.is_truthy(&cond_value) {
                    self.eval(then_branch.clone())
                } else if let Some(else_br) = else_branch {
                    self.eval(else_br.clone())
                } else {
                    Ok(Value::Nothing)
                }
            }

            AstNode::ForStmt { variable, iterable, body } => {
                let iter_value = self.eval_node(iterable)?;

                match iter_value {
                    Value::List(items) => {
                        let mut last = Value::Nothing;
                        for item in items {
                            self.environment.push_scope();
                            self.environment.define(variable.clone(), item);
                            last = self.eval(body.clone())?;
                            self.environment.pop_scope();
                        }
                        Ok(last)
                    }
                    _ => Err(RuntimeError::NotIterable),
                }
            }

            AstNode::Call { callee, args } => {
                self.eval_call(callee, args)
            }

            AstNode::FieldAccess { object, field } => {
                self.eval_field_access(object, field)
            }

            AstNode::BinaryOp { left, op, right } => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }

            // ... more node types ...

            _ => Err(RuntimeError::NotImplemented),
        }
    }

    fn eval_call(&mut self, callee: &AstNode, args: &[AstNode]) -> Result<Value, RuntimeError> {
        // Check if it's a kernel API call (e.g., VGA.write)
        if let AstNode::FieldAccess { object, field } = callee {
            if let AstNode::Ident(module_name) = &**object {
                // This is a kernel API call
                return self.call_kernel_api(module_name, field, args);
            }
        }

        // Otherwise, evaluate the callee to get the function
        let func = self.eval_node(callee)?;

        // Evaluate arguments
        let arg_values: Result<Vec<Value>, RuntimeError> =
            args.iter().map(|arg| self.eval_node(arg)).collect();
        let arg_values = arg_values?;

        match func {
            Value::Chant { params, body, closure } => {
                // Set up new scope with parameters
                self.environment.push_scope();

                for (param, arg) in params.iter().zip(arg_values.iter()) {
                    self.environment.define(param.clone(), arg.clone());
                }

                // Execute body
                let result = self.eval(body)?;

                self.environment.pop_scope();

                Ok(result)
            }

            Value::NativeFunction { func, .. } => {
                func(&arg_values)
            }

            _ => Err(RuntimeError::NotCallable),
        }
    }

    fn call_kernel_api(
        &mut self,
        module: &str,
        function: &str,
        args: &[AstNode]
    ) -> Result<Value, RuntimeError> {
        // Evaluate arguments
        let arg_values: Result<Vec<Value>, RuntimeError> =
            args.iter().map(|arg| self.eval_node(arg)).collect();
        let arg_values = arg_values?;

        // Dispatch to kernel API
        self.kernel_apis.call(module, function, &arg_values)
    }
}
```

### Kernel API Dispatcher

```rust
pub struct KernelApis {
    vga_handle: &'static IrqSafeMutex<VgaWriter>,
    // ... other kernel service handles ...
}

impl KernelApis {
    pub fn call(
        &mut self,
        module: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        match module {
            "VGA" => self.call_vga(function, args),
            "WorldTree" => self.call_worldtree(function, args),
            "Loom" => self.call_loom(function, args),
            "Time" => self.call_time(function, args),
            "Keyboard" => self.call_keyboard(function, args),
            _ => Err(RuntimeError::UnknownModule(module.to_string())),
        }
    }

    fn call_vga(&mut self, function: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match function {
            "write" => {
                if args.len() != 1 {
                    return Err(RuntimeError::WrongArgumentCount);
                }

                let text = match &args[0] {
                    Value::Text(s) => s,
                    Value::Number(n) => &n.to_string(),
                    _ => return Err(RuntimeError::TypeError),
                };

                crate::print!("{}", text);
                Ok(Value::Nothing)
            }

            "writeln" => {
                if args.len() != 1 {
                    return Err(RuntimeError::WrongArgumentCount);
                }

                let text = match &args[0] {
                    Value::Text(s) => s,
                    _ => return Err(RuntimeError::TypeError),
                };

                crate::println!("{}", text);
                Ok(Value::Nothing)
            }

            "clear" => {
                crate::vga_buffer::clear_screen();
                Ok(Value::Nothing)
            }

            // ... more VGA functions ...

            _ => Err(RuntimeError::UnknownFunction(function.to_string())),
        }
    }

    // ... implementations for other modules ...
}
```

## Implementation Plan

### Phase 1: Lexer & Basic Parser (Week 1-2)

**Goals:**
- Implement complete lexer with all tokens
- Parse basic statements (bind, set, if, for)
- Parse expressions (arithmetic, comparison, logical)
- Unit tests for lexer and parser

**Deliverables:**
- `glimmer_weave/lexer.rs` - Token generator
- `glimmer_weave/parser.rs` - AST builder
- `glimmer_weave/ast.rs` - AST node definitions
- Test suite with 50+ test cases

**Example Test:**
```rust
#[test]
fn test_parse_bind_statement() {
    let input = "bind x to 42";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.len(), 1);
    match &ast[0] {
        AstNode::BindStmt { name, value } => {
            assert_eq!(name, "x");
            assert!(matches!(**value, AstNode::Number(42.0)));
        }
        _ => panic!("Expected BindStmt"),
    }
}
```

### Phase 2: Basic Evaluator (Week 3-4)

**Goals:**
- Implement environment (variable storage)
- Evaluate basic expressions and statements
- Implement control flow (if, for)
- Error handling with RuntimeError

**Deliverables:**
- `glimmer_weave/evaluator.rs` - AST interpreter
- `glimmer_weave/value.rs` - Runtime value types
- `glimmer_weave/environment.rs` - Variable scoping
- `glimmer_weave/errors.rs` - Error types

**Example Test:**
```rust
#[test]
fn test_eval_arithmetic() {
    let input = "bind x to 5 + 3";
    let mut eval = Evaluator::new(KernelApis::mock());
    let result = eval.eval_string(input).unwrap();

    assert_eq!(eval.environment.get("x"), Some(&Value::Number(8.0)));
}
```

### Phase 3: Kernel API Integration (Week 5-6)

**Goals:**
- Implement KernelApis struct
- Add VGA module bindings
- Add Loom module bindings (yield, sleep)
- Add Time module bindings
- Test kernel API calls from scripts

**Deliverables:**
- `glimmer_weave/kernel_apis.rs` - API dispatcher
- `glimmer_weave/modules/vga.rs` - VGA bindings
- `glimmer_weave/modules/loom.rs` - Thread bindings
- `glimmer_weave/modules/time.rs` - Time bindings

**Example:**
```rust
// In kernel_apis.rs
impl KernelApis {
    pub fn new() -> Self {
        KernelApis {
            vga_handle: crate::vga_buffer::get_writer(),
        }
    }
}
```

### Phase 4: Advanced Features (Week 7-8)

**Goals:**
- Implement chant definitions (functions)
- Implement closures and captured variables
- Implement pipelines
- Implement pattern matching
- Add World-Tree query syntax (when filesystem is ready)

**Deliverables:**
- Enhanced evaluator with function support
- Pipeline execution engine
- Pattern matching evaluator
- World-Tree query parser (placeholder for future)

### Phase 5: Eldarin Shell Integration (Week 9-10)

**Goals:**
- Integrate Glimmer-Weave evaluator into Eldarin shell
- Implement REPL (Read-Eval-Print-Loop)
- Add command history
- Add tab completion
- Test end-to-end user interaction

**Deliverables:**
- `eldarin/glimmer_repl.rs` - Interactive interpreter
- `eldarin/command_history.rs` - History tracking
- `eldarin/completion.rs` - Tab completion engine
- User documentation with examples

**Example Integration:**
```rust
// In eldarin/mod.rs
pub fn process_command(input: &str) {
    // Check if it's a built-in command
    if input.starts_with("help") {
        show_help();
        return;
    }

    // Otherwise, treat as Glimmer-Weave script
    let mut evaluator = get_global_evaluator();
    match evaluator.eval_string(input) {
        Ok(value) => {
            if !matches!(value, Value::Nothing) {
                println!("{:?}", value);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
```

---

# Part II: The Runic Forge

## Philosophy

The Runic Forge is where ephemeral thoughts become eternal stone. It transforms Rust source code into permanent kernel extensions and high-performance applications that run as part of AethelOS itself.

Unlike Glimmer-Weave, which flows and adapts, the Runic Forge creates **runes** - immutable, verified, optimized code that becomes part of the system's foundation.

## Architecture Overview

The Runic Forge is implemented as an **external privileged service** - not embedded in the kernel, but a separate trusted process with elevated capabilities.

### Why External?

1. **Compilation is Heavy** - Takes significant CPU time and memory
2. **Isolation** - Compiler bugs don't crash the kernel
3. **Upgradability** - Can update compiler without kernel reboot
4. **Security** - Clear boundary between untrusted code and kernel

### Service Model

```
┌──────────────────────────────────────────────────┐
│              User Application                    │
│  (writes Rust code, requests compilation)        │
└──────────────────────────────────────────────────┘
                       ↓ (Nexus message)
┌──────────────────────────────────────────────────┐
│          The Runic Forge (Service)               │
│  • Rust source parser                            │
│  • HIR/MIR generator                             │
│  • Harmonic Analyzer (safety checker)            │
│  • Code generator (x86-64)                       │
│  • Capability resolver                           │
└──────────────────────────────────────────────────┘
                       ↓ (returns binary + metadata)
┌──────────────────────────────────────────────────┐
│            Kernel Loader                         │
│  • Verifies signatures                           │
│  • Maps memory with correct permissions          │
│  • Links capabilities                            │
│  • Registers new rune in system                  │
└──────────────────────────────────────────────────┘
```

## Compilation Pipeline

### Stage 1: Rust Source Parsing

The Forge interprets Rust source code directly (no rustc dependency initially - we'll build our own subset).

**Input:** Rust source file
**Output:** Abstract Syntax Tree (AST)

**Supported Rust Subset (Initial):**
- Functions, structs, enums
- Basic control flow (if, match, loop, while, for)
- No unsafe blocks (capability system replaces unsafe)
- No raw pointers (handles only)
- No allocator (must use kernel allocator API)
- No std library (custom #![no_std] environment)

**Example:**
```rust
// Input: greeter.rune.rs
#![rune]  // Marks this as a rune (kernel extension)

use aethelos::prelude::*;

#[rune_chant]  // Public function (like pub extern)
fn greet(name: &str) -> Result<(), Error> {
    VGA::write(&format!("Mae govannen, {}!", name))?;
    Ok(())
}
```

### Stage 2: HIR (High-Level Intermediate Representation)

AST is lowered to HIR, which is closer to execution semantics.

**Transformations:**
- Desugar for loops → loop + iterators
- Resolve names and paths
- Type inference
- Trait resolution (limited trait system)

### Stage 3: Harmonic Analysis

This is the **safety verification** step. The Harmonic Analyzer ensures the code:
1. **Respects Capabilities** - Only calls APIs it has permission for
2. **Memory Safety** - No use-after-free, no double-free, no data races
3. **Resource Safety** - Doesn't leak handles or exhaust resources
4. **Termination Analysis** - Critical sections don't loop forever
5. **Harmony Metrics** - Estimated CPU usage doesn't exceed threshold

**Harmonic Analysis combines:**
- **Static Analysis** (at compile time):
  - Borrow checking (like rustc)
  - Lifetime analysis
  - Capability requirement tracking
  - Control flow graph analysis

- **Runtime Analysis** (during execution):
  - CPU time tracking
  - Memory allocation tracking
  - Lock hold time monitoring
  - Harmony score calculation

**Example Analysis Output:**
```
Harmonic Analysis Report for greeter.rune
==========================================

✓ Memory Safety: PASS
  - All borrows are valid
  - No lifetimes escape their scope

✓ Capability Safety: PASS
  - Requires: VGA::write
  - Requires: Allocator::alloc (for format!)

✓ Resource Safety: PASS
  - No leaked handles
  - All resources have Drop implementations

✓ Termination: PASS
  - No infinite loops detected
  - Max call depth: 3

✓ Harmony Estimate: 0.92 (Excellent)
  - Estimated CPU time: <1ms
  - Memory usage: ~128 bytes
  - Lock contention: None

VERDICT: SAFE TO LOAD
Required Capabilities:
  - VGA::write
  - Allocator::alloc
```

### Stage 4: MIR (Mid-Level Intermediate Representation)

HIR is lowered to MIR, which is a control-flow graph representation suitable for optimization.

**Optimizations:**
- Constant propagation
- Dead code elimination
- Inline expansion (for small functions)
- Loop unrolling (limited)

### Stage 5: Code Generation

MIR is compiled to x86-64 machine code.

**Output Format:**
```
Rune Binary Format (RBF)
========================

Header:
  Magic: "RUNE" (4 bytes)
  Version: 1 (4 bytes)
  Code Size: N bytes (8 bytes)
  Data Size: M bytes (8 bytes)
  Required Capabilities: (variable length)

Code Section:
  [x86-64 machine code]

Data Section:
  [read-only data, strings, constants]

Metadata Section:
  Entry points: [(name, offset), ...]
  Exported functions: [...]
  Harmony report: {...}
```

### Stage 6: Kernel Loading

The kernel loader verifies and loads the rune:

1. **Signature Verification** - Checks that the rune was compiled by trusted Forge
2. **Capability Grant** - User must approve required capabilities
3. **Memory Mapping** - Allocates memory and maps code (read-exec), data (read-only)
4. **Symbol Linking** - Links rune's calls to kernel APIs
5. **Registration** - Adds rune to system's loaded runes table

**Example Loading:**
```glimmer-weave
# Load a compiled rune
bind rune to RunicForge.load("greeter.rune")

# Grant capabilities (user is prompted)
should rune.needs_approval() then
    VGA.writeln("Rune requires capabilities:")
    for each cap in rune.capabilities() then
        VGA.writeln("  - " + cap)
    end

    bind answer to Keyboard.read_line()
    should answer is "yes" then
        rune.grant_all()
    end
end

# Call rune function
rune.call("greet", ["Elara"])
```

## Capability-Based Linking

Instead of traditional dynamic linking, runes use **capability-based method calls**.

### How It Works

1. **Rune declares what it needs:**
   ```rust
   #[requires(VGA::write)]
   #[requires(Allocator::alloc)]
   fn my_function() { ... }
   ```

2. **At load time, user grants capabilities:**
   ```
   Grant VGA::write to greeter.rune? [yes/no]: yes
   Grant Allocator::alloc to greeter.rune? [yes/no]: yes
   ```

3. **Kernel creates capability tokens:**
   ```rust
   struct CapabilityToken {
       id: u64,              // Unforgeable ID
       granted_to: RuneId,   // Which rune owns this
       permission: Permission, // What it can do
   }
   ```

4. **Rune calls API through kernel:**
   ```rust
   // Inside compiled rune:
   kernel_call(CAPABILITY_VGA_WRITE, "Hello, world!");
   ```

5. **Kernel checks token before executing:**
   ```rust
   fn kernel_call(cap_id: u64, args: &[u8]) {
       let cap = lookup_capability(cap_id);
       if cap.granted_to == current_rune() {
           match cap.permission {
               Permission::VGAWrite => vga_write(args),
               _ => deny(),
           }
       } else {
           deny(); // Unauthorized!
       }
   }
   ```

### Capability Delegation

Runes can delegate their capabilities to child threads:

```rust
#[rune_chant]
fn spawn_worker() {
    // Delegate VGA::write to new thread
    let thread = Loom::spawn_with_caps(
        worker_function,
        &[my_capabilities().vga_write]
    );
}
```

## Package Manager (Rune Repository)

The Runic Forge includes a package manager for sharing runes.

### Rune Manifest Format

```toml
[rune]
name = "greeter"
version = "1.0.0"
authors = ["Elara"]
description = "A friendly greeting rune"

[dependencies]
# No external dependencies for now

[capabilities]
required = ["VGA::write", "Allocator::alloc"]

[build]
entry_point = "src/lib.rs"
```

### Repository Structure

```
World-Tree Query:
essence:RunePackage AND tag:category=utility

Each package is a Scroll with:
- name: "greeter"
- essence: "RunePackage"
- content: compiled .rune binary
- metadata:
  - version: "1.0.0"
  - author: "Elara"
  - capabilities: ["VGA::write"]
  - harmonic_score: 0.92
  - source_hash: sha256(source code)
```

### Installing a Rune

```glimmer-weave
# Search for runes
bind packages to WorldTree.seek(
    essence: "RunePackage",
    tag: "category=utility"
)

# Install a rune
RunicForge.install("greeter", version: "1.0.0")

# Loads the rune, verifies it, prompts for capabilities
```

## Self-Hosting Goal

Eventually, the Runic Forge itself will be a rune compiled by an older version of itself. This creates a **self-evolving compiler**.

**Bootstrap Path:**
1. **Phase 1:** Simple interpreter-based Forge (written in Rust, compiled with rustc externally)
2. **Phase 2:** Forge v1 can compile simple Rust subsets
3. **Phase 3:** Forge v2 is a rune compiled by Forge v1
4. **Phase 4:** Forge v3+ are runes compiled by previous Forge versions

This allows AethelOS to evolve its own compiler over time.

## Implementation Plan

### Phase 1: Basic Rust Parser (Month 1-2)

**Goals:**
- Parse basic Rust syntax (functions, structs, expressions)
- Build AST representation
- Handle no_std subset

**Deliverables:**
- `runic_forge/parser.rs` - Rust parser
- `runic_forge/ast.rs` - AST definitions
- Test suite with example runes

### Phase 2: HIR & Type Checking (Month 3-4)

**Goals:**
- Lower AST to HIR
- Implement type inference
- Basic trait resolution
- Name resolution

**Deliverables:**
- `runic_forge/hir.rs` - HIR representation
- `runic_forge/typeck.rs` - Type checker
- `runic_forge/resolve.rs` - Name resolver

### Phase 3: Harmonic Analyzer (Month 5-6)

**Goals:**
- Implement borrow checker
- Lifetime analysis
- Capability requirement extraction
- Basic harmony metrics

**Deliverables:**
- `runic_forge/borrow_check.rs` - Borrow checker
- `runic_forge/harmony.rs` - Harmony analyzer
- Safety reports with detailed output

### Phase 4: Code Generation (Month 7-8)

**Goals:**
- MIR generation
- Basic optimizations
- x86-64 code generation
- Rune binary format

**Deliverables:**
- `runic_forge/mir.rs` - MIR representation
- `runic_forge/codegen.rs` - x86-64 backend
- `runic_forge/rune_format.rs` - Binary format

### Phase 5: Kernel Loader (Month 9-10)

**Goals:**
- Rune loader in kernel
- Capability granting system
- Symbol linking
- Runtime verification

**Deliverables:**
- `heartwood/rune_loader.rs` - Loader implementation
- `heartwood/capabilities.rs` - Capability system
- Integration with Loom (runes run as threads)

### Phase 6: Package Manager (Month 11-12)

**Goals:**
- Rune manifest parser
- Package installation
- Dependency resolution (future)
- Integration with World-Tree

**Deliverables:**
- `runic_forge/package.rs` - Package manager
- `runic_forge/manifest.rs` - Manifest parser
- CLI for managing runes

---

# Part III: Integration & Ecosystem

## How Glimmer-Weave and Runic Forge Work Together

### Development Workflow

```
1. Prototype in Glimmer-Weave
   ↓
   User writes quick script to test an idea
   "Does this algorithm work? Let me try in Glimmer..."

2. Refine and Iterate
   ↓
   Script works! But it's slow because it's interpreted.

3. Forge into a Rune
   ↓
   Rewrite core algorithm in Rust, compile with Runic Forge.
   Now it's fast, safe, and permanent.

4. Use Rune from Glimmer
   ↓
   Call the compiled rune from Glimmer-Weave scripts.
   Best of both worlds: flexibility + performance.
```

### Example: Building a Text Editor

**Step 1: Prototype in Glimmer-Weave**
```glimmer-weave
# editor.glim - Quick prototype
bind buffer to []
bind cursor to 0

chant insert_char(ch) then
    buffer.insert(cursor, ch)
    set cursor to cursor + 1
end

chant render() then
    VGA.clear()
    for each line in buffer then
        VGA.writeln(line)
    end
end

# Main loop
loop
    bind key to Keyboard.read_key()
    should key is "Enter" then
        break
    end
    insert_char(key)
    render()
end
```

**Step 2: Performance Bottleneck Identified**
> "Rendering is slow. Re-drawing the entire screen on every keystroke."

**Step 3: Forge a Rune for Rendering**
```rust
// editor_render.rune.rs
#![rune]

use aethelos::vga::VGA;

#[rune_chant]
fn render_line(line_num: usize, text: &str, dirty: bool) -> Result<(), Error> {
    if dirty {
        // Only render lines that changed
        VGA::set_cursor(line_num, 0)?;
        VGA::write_line(text)?;
    }
    Ok(())
}
```

**Step 4: Use Rune from Glimmer**
```glimmer-weave
# Load the optimized rune
bind renderer to RunicForge.load("editor_render.rune")

chant render_optimized() then
    for each i, line in enumerate(buffer) then
        renderer.call("render_line", [i, line, buffer.dirty[i]])
    end
end

# Now use the fast version
render_optimized()
```

## Self-Evolving System

Over time, AethelOS will **evolve itself**:

1. **Glimmer-Weave provides rapid experimentation**
   - Users write scripts to try new ideas
   - Scripts that prove useful are shared in World-Tree

2. **Runic Forge crystalizes good ideas**
   - Popular scripts are rewritten as runes
   - Runes become part of the core system

3. **The OS grows organically**
   - New capabilities emerge from community
   - System becomes richer over time
   - Ancient Runes library expands

**Example Evolution:**
```
Year 1: VGA module only supports text mode
  ↓
User writes Glimmer script for simple graphics
  ↓
Script is popular, forged into "framebuffer.rune"
  ↓
Year 2: Framebuffer is now standard library
  ↓
Another user builds on it, creates "sprite_engine.rune"
  ↓
Year 3: AethelOS has a full graphics stack, all community-built!
```

## Standard Library Growth

**Current (Year 1):**
- VGA text output
- Basic keyboard input
- Simple threading
- File storage (World-Tree)

**Future (Year 2-3):**
- Graphics (framebuffer, sprites, fonts)
- Networking (TCP/IP stack as runes)
- Audio (sound synthesis runes)
- Advanced UI (widget library)

**Future (Year 5+):**
- Full compiler toolchain (runes compiling runes)
- Package ecosystem (hundreds of community runes)
- Self-hosting development environment
- OS bootstraps itself from source

## Harmony-Driven Evolution

The system **rewards good citizenship**:

- **High-harmony runes** get priority:
  - Faster scheduling
  - More resources
  - Featured in package search

- **Low-harmony runes** get throttled:
  - Slower scheduling
  - Resource limits
  - Warning labels in package manager

This creates a **natural selection** of code - efficient, cooperative code thrives.

---

## Summary

**Glimmer-Weave** is the voice - flexible, expressive, immediate.
**Runic Forge** is the stone - permanent, verified, optimized.

Together, they create a system that can **think** (interpret scripts) and **remember** (compile to native code), evolving from ephemeral thought to eternal wisdom.

> *"First, the word. Then, the rune. Finally, the world."*

---

## Appendix: Example Glimmer-Weave Programs

### Hello World
```glimmer-weave
VGA.writeln("Mae govannen, AethelOS!")
```

### Fibonacci Sequence
```glimmer-weave
chant fib(n) then
    should n <= 1 then
        yield n
    otherwise
        yield fib(n - 1) + fib(n - 2)
    end
end

for i in range(0, 10) then
    VGA.writeln(fib(i))
end
```

### File Search
```glimmer-weave
bind scrolls to WorldTree.seek(
    essence: "Scroll",
    creator: "Elara",
    created: after 2025-01-01
)

VGA.writeln("Found " + scrolls.length + " scrolls:")
for each scroll in scrolls then
    VGA.writeln("  - " + scroll.name + " (" + scroll.created + ")")
end
```

### Simple Shell Command
```glimmer-weave
# Command: list
chant cmd_list() then
    bind scrolls to WorldTree.seek(essence: "Scroll")
    for each scroll in scrolls then
        VGA.writeln(scroll.name)
    end
end

# Command: greet
chant cmd_greet(name) then
    VGA.writeln("Mae govannen, " + name + "!")
end

# Register commands
Eldarin.register("list", cmd_list)
Eldarin.register("greet", cmd_greet)
```

### Thread Spawning
```glimmer-weave
chant worker() then
    for i in range(1, 100) then
        VGA.write(".")
        Loom.sleep(100)
    end
end

bind thread to Loom.spawn(priority: "Low", chant: worker)
VGA.writeln("Worker thread started: " + thread.id)
```

---

**End of Document**
