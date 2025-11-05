//! # Abstract Syntax Tree (AST)
//!
//! Defines the structure of parsed Glimmer-Weave programs.
//!
//! The AST represents the syntactic structure of Glimmer-Weave code,
//! capturing statements, expressions, and their relationships.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// A node in the Abstract Syntax Tree
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    // === Statements ===

    /// Immutable binding: `bind x to 42`
    BindStmt {
        name: String,
        value: Box<AstNode>,
    },

    /// Mutable variable: `weave counter as 0`
    WeaveStmt {
        name: String,
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

    /// Loop: `for each x in list then ... end`
    ForStmt {
        variable: String,
        iterable: Box<AstNode>,
        body: Vec<AstNode>,
    },

    /// Function definition: `chant greet(name) then ... end`
    ChantDef {
        name: String,
        params: Vec<String>,
        body: Vec<AstNode>,
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

    /// List literal: `[1, 2, 3]`
    List(Vec<AstNode>),

    /// Map literal: `{name: "Elara", age: 42}`
    Map(Vec<(String, AstNode)>),

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
