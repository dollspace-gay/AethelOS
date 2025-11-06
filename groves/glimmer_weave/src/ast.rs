//! # Abstract Syntax Tree (AST)
//!
//! Defines the structure of parsed Glimmer-Weave programs.
//!
//! The AST represents the syntactic structure of Glimmer-Weave code,
//! capturing statements, expressions, and their relationships.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Type annotation in the AST (syntactic representation)
///
/// This is the syntactic form of types as they appear in source code.
/// The semantic analyzer converts these to semantic::Type for type checking.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// Simple type name: `Number`, `Text`, `Truth`
    Named(String),
    /// List type: `List<Number>`
    List(Box<TypeAnnotation>),
    /// Map type: `Map`
    Map,
    /// Function type: `Function<(Number, Text) -> Truth>`
    Function {
        param_types: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
    },
    /// Optional/nullable type: `Number?` (future feature)
    Optional(Box<TypeAnnotation>),
}

/// Function parameter with optional type annotation
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub typ: Option<TypeAnnotation>,
}

/// Struct field definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub typ: TypeAnnotation,
}

impl Parameter {
    /// Create a parameter without type annotation (for backward compatibility)
    pub fn untyped(name: String) -> Self {
        Parameter { name, typ: None }
    }

    /// Create a parameter with type annotation
    pub fn typed(name: String, typ: TypeAnnotation) -> Self {
        Parameter { name, typ: Some(typ) }
    }
}

/// A node in the Abstract Syntax Tree
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    // === Statements ===

    /// Immutable binding: `bind x to 42` or `bind x: Number to 42`
    BindStmt {
        name: String,
        typ: Option<TypeAnnotation>,
        value: Box<AstNode>,
    },

    /// Mutable variable: `weave counter as 0` or `weave counter: Number as 0`
    WeaveStmt {
        name: String,
        typ: Option<TypeAnnotation>,
        value: Box<AstNode>,
    },

    /// Mutation: `set counter to 10`
    SetStmt {
        name: String,
        value: Box<AstNode>,
    },

    /// Conditional: `should x > 5 then ... otherwise ... end`
    IfStmt {
        condition: Box<AstNode>,
        then_branch: Vec<AstNode>,
        else_branch: Option<Vec<AstNode>>,
    },

    /// Bounded loop: `for each x in list then ... end`
    ForStmt {
        variable: String,
        iterable: Box<AstNode>,
        body: Vec<AstNode>,
    },

    /// Unbounded loop: `whilst condition then ... end`
    WhileStmt {
        condition: Box<AstNode>,
        body: Vec<AstNode>,
    },

    /// Function definition: `chant greet(name) then ... end`
    /// or with types: `chant factorial(n: Number) -> Number then ... end`
    ChantDef {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<TypeAnnotation>,
        body: Vec<AstNode>,
    },

    /// Struct definition: `form Person with name as Text age as Number end`
    FormDef {
        name: String,
        fields: Vec<StructField>,
    },

    /// Return statement: `yield result`
    YieldStmt {
        value: Box<AstNode>,
    },

    /// Pattern matching: `match x with when 1 then ... end`
    MatchStmt {
        value: Box<AstNode>,
        arms: Vec<MatchArm>,
    },

    /// Error handling: `attempt ... harmonize on Error then ... end`
    AttemptStmt {
        body: Vec<AstNode>,
        handlers: Vec<ErrorHandler>,
    },

    /// Capability request: `request VGA.write with justification "message"`
    RequestStmt {
        capability: Box<AstNode>,
        justification: String,
    },

    // === Expressions ===

    /// Numeric literal: `42`, `3.14`
    Number(f64),

    /// String literal: `"hello"`
    Text(String),

    /// Boolean literal: `true`, `false`
    Truth(bool),

    /// Null/void value: `nothing`
    Nothing,

    /// Variable reference: `x`, `counter`
    Ident(String),

    /// Triumph value: `Triumph(42)` (successful Outcome)
    Triumph(Box<AstNode>),

    /// Mishap value: `Mishap("error")` (failed Outcome)
    Mishap(Box<AstNode>),

    /// Present value: `Present(42)` (Maybe with value)
    Present(Box<AstNode>),

    /// Absent value: `Absent` (Maybe without value)
    Absent,

    /// List literal: `[1, 2, 3]`
    List(Vec<AstNode>),

    /// Map literal: `{name: "Elara", age: 42}`
    Map(Vec<(String, AstNode)>),

    /// Struct literal: `Person { name: "Alice", age: 30 }`
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, AstNode)>,
    },

    /// Binary operation: `x + y`, `a > b`
    BinaryOp {
        left: Box<AstNode>,
        op: BinaryOperator,
        right: Box<AstNode>,
    },

    /// Unary operation: `not x`, `-y`
    UnaryOp {
        op: UnaryOperator,
        operand: Box<AstNode>,
    },

    /// Function call: `greet("Elara")`, `VGA.write("Hello")`
    Call {
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },

    /// Field access: `person.name`, `VGA.write`
    FieldAccess {
        object: Box<AstNode>,
        field: String,
    },

    /// Index access: `list[0]`
    IndexAccess {
        object: Box<AstNode>,
        index: Box<AstNode>,
    },

    /// Range: `range(1, 10)`
    Range {
        start: Box<AstNode>,
        end: Box<AstNode>,
    },

    /// Pipeline: `x | filter by y > 5 | take 10`
    Pipeline {
        stages: Vec<AstNode>,
    },

    /// Query expression: `seek where essence is "Scroll"`
    SeekExpr {
        conditions: Vec<QueryCondition>,
    },

    /// Expression statement (for side effects)
    ExprStmt(Box<AstNode>),

    /// Block of statements
    Block(Vec<AstNode>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %

    // Comparison
    Equal,    // is
    NotEqual, // is not
    Greater,  // >
    Less,     // <
    GreaterEq, // >=
    LessEq,   // <=

    // Logical
    And,      // and
    Or,       // or
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,     // not
    Negate,  // -
}

/// Match arm: `when pattern then body`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Vec<AstNode>,
}

/// Pattern for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal pattern: `when 42 then ...`
    Literal(AstNode),
    /// Variable binding pattern: `when x then ...`
    Ident(String),
    /// Wildcard pattern: `otherwise`
    Wildcard,
    /// Enum pattern: `when Triumph(x) then ...` or `when Absent then ...`
    Enum {
        variant: String,  // "Triumph", "Mishap", "Present", "Absent"
        inner: Option<Box<Pattern>>,  // The inner pattern (if any)
    },
}

/// Error handler: `harmonize on ErrorType then ...`
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorHandler {
    pub error_type: String,
    pub body: Vec<AstNode>,
}

/// Query condition for seek expressions
#[derive(Debug, Clone, PartialEq)]
pub struct QueryCondition {
    pub field: String,
    pub operator: QueryOperator,
    pub value: Box<AstNode>,
}

/// Query operators for World-Tree queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOperator {
    Is,           // is
    IsNot,        // is not
    Greater,      // >
    Less,         // <
    GreaterEq,    // >=
    LessEq,       // <=
    After,        // after (temporal)
    Before,       // before (temporal)
}

impl AstNode {
    /// Check if this node is a statement
    pub fn is_statement(&self) -> bool {
        matches!(
            self,
            AstNode::BindStmt { .. }
                | AstNode::WeaveStmt { .. }
                | AstNode::SetStmt { .. }
                | AstNode::IfStmt { .. }
                | AstNode::ForStmt { .. }
                | AstNode::WhileStmt { .. }
                | AstNode::ChantDef { .. }
                | AstNode::YieldStmt { .. }
                | AstNode::MatchStmt { .. }
                | AstNode::AttemptStmt { .. }
                | AstNode::RequestStmt { .. }
                | AstNode::ExprStmt(_)
        )
    }

    /// Check if this node is an expression
    pub fn is_expression(&self) -> bool {
        !self.is_statement()
    }
}

impl BinaryOperator {
    /// Get the precedence of this operator (higher = tighter binding)
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Greater
            | BinaryOperator::Less
            | BinaryOperator::GreaterEq
            | BinaryOperator::LessEq => 3,
            BinaryOperator::Add | BinaryOperator::Sub => 4,
            BinaryOperator::Mul | BinaryOperator::Div | BinaryOperator::Mod => 5,
        }
    }
}
