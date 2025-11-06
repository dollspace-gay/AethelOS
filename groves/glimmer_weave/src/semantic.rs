//! # Semantic Analyzer
//!
//! Validates Glimmer-Weave programs before execution or compilation.
//!
//! The semantic analyzer performs:
//! - **Name resolution**: Checks that all variables/functions are defined before use
//! - **Type checking**: Validates type compatibility in operations and assignments
//! - **Scope analysis**: Tracks variable scopes and detects shadowing
//! - **Function arity checking**: Validates function calls have correct argument counts
//!
//! This catches errors early, before runtime or code generation, providing
//! better error messages and preventing invalid programs from executing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use crate::ast::*;

/// Types in the Glimmer-Weave type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Numeric type
    Number,
    /// String type
    Text,
    /// Boolean type
    Truth,
    /// Null/void type
    Nothing,
    /// List of values (homogeneous or heterogeneous)
    List(Box<Type>),  // Box<Type::Any> for heterogeneous lists
    /// Map from string keys to values
    Map,
    /// Function type (param types, return type)
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    /// Capability type
    Capability,
    /// Range type
    Range,
    /// Unknown/unresolved type
    Unknown,
    /// Any type (for dynamic typing)
    Any,
}

impl Type {
    /// Check if this type is compatible with another type
    pub fn is_compatible(&self, other: &Type) -> bool {
        match (self, other) {
            // Exact match
            (a, b) if a == b => true,
            // Any type is compatible with everything
            (Type::Any, _) | (_, Type::Any) => true,
            // Unknown can be anything (used during type inference)
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            // Lists are compatible if element types match
            (Type::List(a), Type::List(b)) => a.is_compatible(b),
            // Otherwise incompatible
            _ => false,
        }
    }

    /// Get a human-readable name for this type
    pub fn name(&self) -> &str {
        match self {
            Type::Number => "Number",
            Type::Text => "Text",
            Type::Truth => "Truth",
            Type::Nothing => "Nothing",
            Type::List(_) => "List",
            Type::Map => "Map",
            Type::Function { .. } => "Function",
            Type::Capability => "Capability",
            Type::Range => "Range",
            Type::Unknown => "Unknown",
            Type::Any => "Any",
        }
    }
}

/// Semantic errors detected during analysis
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// Variable used before definition
    UndefinedVariable(String),
    /// Function called but not defined
    UndefinedFunction(String),
    /// Variable defined multiple times in same scope
    DuplicateDefinition(String),
    /// Type mismatch in operation
    TypeError {
        expected: String,
        got: String,
        context: String,
    },
    /// Function called with wrong number of arguments
    ArityMismatch {
        function: String,
        expected: usize,
        got: usize,
    },
    /// Attempt to mutate immutable binding
    ImmutableBinding(String),
    /// Return statement outside function
    ReturnOutsideFunction,
    /// Invalid operation on type
    InvalidOperation {
        operation: String,
        operand_type: String,
    },
    /// Match expression is not exhaustive (doesn't cover all cases)
    NonExhaustiveMatch {
        message: String,
    },
}

/// Symbol in the symbol table
#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    typ: Type,
    mutable: bool,
    defined: bool,  // For forward declarations
}

/// Scope in the symbol table
#[derive(Debug, Clone)]
struct Scope {
    symbols: BTreeMap<String, Symbol>,
    parent: Option<usize>,  // Index of parent scope
}

impl Scope {
    fn new(parent: Option<usize>) -> Self {
        Scope {
            symbols: BTreeMap::new(),
            parent,
        }
    }

    fn define(&mut self, name: String, typ: Type, mutable: bool) {
        self.symbols.insert(name.clone(), Symbol {
            name,
            typ,
            mutable,
            defined: true,
        });
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }
}

/// Symbol table for tracking variable scopes
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        let global_scope = Scope::new(None);
        SymbolTable {
            scopes: vec![global_scope],
            current_scope: 0,
        }
    }

    /// Enter a new scope
    pub fn push_scope(&mut self) {
        let new_scope = Scope::new(Some(self.current_scope));
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    /// Exit current scope
    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Define a symbol in the current scope
    pub fn define(&mut self, name: String, typ: Type, mutable: bool) -> Result<(), SemanticError> {
        // Check for duplicate in current scope
        if self.scopes[self.current_scope].lookup(&name).is_some() {
            return Err(SemanticError::DuplicateDefinition(name));
        }

        self.scopes[self.current_scope].define(name, typ, mutable);
        Ok(())
    }

    /// Lookup a symbol in current scope and parent scopes
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(symbol) = self.scopes[scope_idx].lookup(name) {
                return Some(symbol);
            }

            // Check parent scope
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                return None;
            }
        }
    }
}

/// Semantic analyzer state
pub struct SemanticAnalyzer {
    symbol_table: SymbolTable,
    in_function: bool,
    errors: Vec<SemanticError>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        let mut analyzer = SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            in_function: false,
            errors: Vec::new(),
        };

        // Register builtin functions
        analyzer.register_builtins();

        analyzer
    }

    /// Register builtin runtime library functions
    fn register_builtins(&mut self) {
        // String functions
        let _ = self.symbol_table.define(
            "length".to_string(),
            Type::Function {
                params: vec![Type::Text],
                return_type: Box::new(Type::Number),
            },
            false,
        );

        let _ = self.symbol_table.define(
            "upper".to_string(),
            Type::Function {
                params: vec![Type::Text],
                return_type: Box::new(Type::Text),
            },
            false,
        );

        let _ = self.symbol_table.define(
            "lower".to_string(),
            Type::Function {
                params: vec![Type::Text],
                return_type: Box::new(Type::Text),
            },
            false,
        );

        // Math functions
        let _ = self.symbol_table.define(
            "sqrt".to_string(),
            Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            },
            false,
        );

        let _ = self.symbol_table.define(
            "pow".to_string(),
            Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            },
            false,
        );

        // Type conversion
        let _ = self.symbol_table.define(
            "to_text".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Text),
            },
            false,
        );

        let _ = self.symbol_table.define(
            "to_number".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Number),
            },
            false,
        );

        // List functions
        let _ = self.symbol_table.define(
            "list_length".to_string(),
            Type::Function {
                params: vec![Type::List(Box::new(Type::Any))],
                return_type: Box::new(Type::Number),
            },
            false,
        );

        // Map functions
        let _ = self.symbol_table.define(
            "map_keys".to_string(),
            Type::Function {
                params: vec![Type::Map],
                return_type: Box::new(Type::List(Box::new(Type::Text))),
            },
            false,
        );

        // Add more builtins as needed...
    }

    /// Analyze a program (list of statements)
    pub fn analyze(&mut self, nodes: &[AstNode]) -> Result<(), Vec<SemanticError>> {
        for node in nodes {
            self.analyze_node(node);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Analyze a single AST node
    fn analyze_node(&mut self, node: &AstNode) -> Type {
        match node {
            // === Literals ===
            AstNode::Number(_) => Type::Number,
            AstNode::Text(_) => Type::Text,
            AstNode::Truth(_) => Type::Truth,
            AstNode::Nothing => Type::Nothing,

            // === Outcome/Maybe Constructors ===
            AstNode::Triumph(_value) => Type::Any, // TODO: proper Outcome<T, E> type
            AstNode::Mishap(_value) => Type::Any, // TODO: proper Outcome<T, E> type
            AstNode::Present(_value) => Type::Any, // TODO: proper Maybe<T> type
            AstNode::Absent => Type::Any, // TODO: proper Maybe<T> type

            // === Variables ===
            AstNode::Ident(name) => {
                if let Some(symbol) = self.symbol_table.lookup(name) {
                    symbol.typ.clone()
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                    Type::Unknown
                }
            }

            // === Statements ===
            AstNode::BindStmt { name, typ, value } => {
                let value_type = self.analyze_node(value);

                // If type annotation is provided, check compatibility
                let declared_type = if let Some(type_ann) = typ {
                    let t = Self::convert_type_annotation(type_ann);
                    // Check value matches declared type
                    if !t.is_compatible(&value_type) {
                        self.errors.push(SemanticError::TypeError {
                            expected: t.name().to_string(),
                            got: value_type.name().to_string(),
                            context: format!("binding '{}'", name),
                        });
                    }
                    t
                } else {
                    value_type
                };

                if let Err(e) = self.symbol_table.define(name.clone(), declared_type, false) {
                    self.errors.push(e);
                }
                Type::Nothing
            }

            AstNode::WeaveStmt { name, typ, value } => {
                let value_type = self.analyze_node(value);

                // If type annotation is provided, check compatibility
                let declared_type = if let Some(type_ann) = typ {
                    let t = Self::convert_type_annotation(type_ann);
                    // Check value matches declared type
                    if !t.is_compatible(&value_type) {
                        self.errors.push(SemanticError::TypeError {
                            expected: t.name().to_string(),
                            got: value_type.name().to_string(),
                            context: format!("weaving '{}'", name),
                        });
                    }
                    t
                } else {
                    value_type
                };

                if let Err(e) = self.symbol_table.define(name.clone(), declared_type, true) {
                    self.errors.push(e);
                }
                Type::Nothing
            }

            AstNode::SetStmt { name, value } => {
                // Check variable exists and is mutable
                let symbol_info = self.symbol_table.lookup(name).map(|s| (s.typ.clone(), s.mutable));

                if let Some((expected_type, is_mutable)) = symbol_info {
                    if !is_mutable {
                        self.errors.push(SemanticError::ImmutableBinding(name.clone()));
                    }
                    let value_type = self.analyze_node(value);
                    if !expected_type.is_compatible(&value_type) {
                        self.errors.push(SemanticError::TypeError {
                            expected: expected_type.name().to_string(),
                            got: value_type.name().to_string(),
                            context: format!("assignment to '{}'", name),
                        });
                    }
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                }
                Type::Nothing
            }

            AstNode::ChantDef { name, params, return_type, body } => {
                // Extract parameter types from annotations
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| {
                        p.typ
                            .as_ref()
                            .map(|t| Self::convert_type_annotation(t))
                            .unwrap_or(Type::Any)
                    })
                    .collect();

                // Extract return type from annotation
                let ret_type = return_type
                    .as_ref()
                    .map(|t| Self::convert_type_annotation(t))
                    .unwrap_or(Type::Any);

                // Define function in current scope
                let func_type = Type::Function {
                    params: param_types.clone(),
                    return_type: Box::new(ret_type),
                };

                if let Err(e) = self.symbol_table.define(name.clone(), func_type, false) {
                    self.errors.push(e);
                }

                // Analyze function body in new scope
                self.symbol_table.push_scope();
                self.in_function = true;

                // Define parameters with their types
                for (param, param_type) in params.iter().zip(param_types.iter()) {
                    let _ = self.symbol_table.define(param.name.clone(), param_type.clone(), false);
                }

                // Analyze body
                for stmt in body {
                    self.analyze_node(stmt);
                }

                self.in_function = false;
                self.symbol_table.pop_scope();

                Type::Nothing
            }

            AstNode::FormDef { name, fields: _ } => {
                // Define struct type in current scope
                // For now, we'll use Type::Any as a placeholder
                // In a more complete implementation, we'd have a Type::Struct variant
                if let Err(e) = self.symbol_table.define(name.clone(), Type::Any, false) {
                    self.errors.push(e);
                }
                Type::Nothing
            }

            AstNode::StructLiteral { struct_name, fields: _ } => {
                // Check that the struct type exists
                if self.symbol_table.lookup(struct_name).is_none() {
                    self.errors.push(SemanticError::UndefinedVariable(struct_name.clone()));
                }
                // Return Any for now - in future could be Type::Struct(struct_name)
                Type::Any
            }

            AstNode::YieldStmt { value } => {
                if !self.in_function {
                    self.errors.push(SemanticError::ReturnOutsideFunction);
                }
                self.analyze_node(value)
            }

            // === Control Flow ===
            AstNode::IfStmt { condition, then_branch, else_branch } => {
                let cond_type = self.analyze_node(condition);
                // Condition can be any type (truthiness)

                // Analyze branches
                self.symbol_table.push_scope();
                for stmt in then_branch {
                    self.analyze_node(stmt);
                }
                self.symbol_table.pop_scope();

                if let Some(else_stmts) = else_branch {
                    self.symbol_table.push_scope();
                    for stmt in else_stmts {
                        self.analyze_node(stmt);
                    }
                    self.symbol_table.pop_scope();
                }

                Type::Nothing
            }

            AstNode::ForStmt { variable, iterable, body } => {
                let iter_type = self.analyze_node(iterable);

                // Check iterable is List or Range
                match iter_type {
                    Type::List(_) | Type::Range | Type::Any | Type::Unknown => {},
                    _ => {
                        self.errors.push(SemanticError::TypeError {
                            expected: "List or Range".to_string(),
                            got: iter_type.name().to_string(),
                            context: "for loop iterable".to_string(),
                        });
                    }
                }

                // Analyze body in new scope with loop variable
                self.symbol_table.push_scope();
                let _ = self.symbol_table.define(variable.clone(), Type::Any, false);

                for stmt in body {
                    self.analyze_node(stmt);
                }

                self.symbol_table.pop_scope();
                Type::Nothing
            }

            AstNode::WhileStmt { condition, body } => {
                // Analyze condition (should evaluate to something truthy)
                let _cond_type = self.analyze_node(condition);
                // Accept any type for condition (will be checked at runtime via is_truthy)

                // Analyze body in new scope
                self.symbol_table.push_scope();

                for stmt in body {
                    self.analyze_node(stmt);
                }

                self.symbol_table.pop_scope();
                Type::Nothing
            }

            // === Binary Operations ===
            AstNode::BinaryOp { left, op, right } => {
                let left_type = self.analyze_node(left);
                let right_type = self.analyze_node(right);

                match op {
                    BinaryOperator::Add => {
                        // Add works for Number + Number (arithmetic) and Text + Text (concatenation)
                        match (&left_type, &right_type) {
                            // Number + Number => Number
                            (Type::Number, Type::Number) => Type::Number,
                            // Text + Text => Text
                            (Type::Text, Type::Text) => Type::Text,
                            // Any/Unknown can be either
                            (Type::Any, _) | (_, Type::Any) => Type::Any,
                            (Type::Unknown, _) | (_, Type::Unknown) => Type::Unknown,
                            // Mixed types are errors
                            _ => {
                                self.errors.push(SemanticError::TypeError {
                                    expected: "Number or Text".to_string(),
                                    got: format!("{} + {}", left_type.name(), right_type.name()),
                                    context: "addition/concatenation requires matching types".to_string(),
                                });
                                Type::Unknown
                            }
                        }
                    }

                    BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div | BinaryOperator::Mod => {
                        // Other arithmetic requires numbers only
                        if !matches!(left_type, Type::Number | Type::Any | Type::Unknown) {
                            self.errors.push(SemanticError::TypeError {
                                expected: "Number".to_string(),
                                got: left_type.name().to_string(),
                                context: format!("left operand of {:?}", op),
                            });
                        }
                        if !matches!(right_type, Type::Number | Type::Any | Type::Unknown) {
                            self.errors.push(SemanticError::TypeError {
                                expected: "Number".to_string(),
                                got: right_type.name().to_string(),
                                context: format!("right operand of {:?}", op),
                            });
                        }
                        Type::Number
                    }

                    BinaryOperator::Equal | BinaryOperator::NotEqual |
                    BinaryOperator::Less | BinaryOperator::Greater |
                    BinaryOperator::LessEq | BinaryOperator::GreaterEq => {
                        // Comparison operators return boolean
                        Type::Truth
                    }

                    BinaryOperator::And | BinaryOperator::Or => {
                        // Logical operators (any type can be truthy)
                        Type::Truth
                    }
                }
            }

            // === Unary Operations ===
            AstNode::UnaryOp { op, operand } => {
                let operand_type = self.analyze_node(operand);

                match op {
                    UnaryOperator::Negate => {
                        if !matches!(operand_type, Type::Number | Type::Any | Type::Unknown) {
                            self.errors.push(SemanticError::TypeError {
                                expected: "Number".to_string(),
                                got: operand_type.name().to_string(),
                                context: "negation operand".to_string(),
                            });
                        }
                        Type::Number
                    }

                    UnaryOperator::Not => {
                        // Logical not (any type can be truthy)
                        Type::Truth
                    }
                }
            }

            // === Function Calls ===
            AstNode::Call { callee, args } => {
                let func_type = self.analyze_node(callee);

                // Analyze argument types
                let arg_types: Vec<Type> = args.iter()
                    .map(|arg| self.analyze_node(arg))
                    .collect();

                match func_type {
                    Type::Function { params, return_type } => {
                        // Check arity
                        if params.len() != arg_types.len() {
                            if let AstNode::Ident(name) = &**callee {
                                self.errors.push(SemanticError::ArityMismatch {
                                    function: name.clone(),
                                    expected: params.len(),
                                    got: arg_types.len(),
                                });
                            }
                        }

                        // Check parameter types
                        for (i, (param_type, arg_type)) in params.iter().zip(arg_types.iter()).enumerate() {
                            if !param_type.is_compatible(arg_type) {
                                self.errors.push(SemanticError::TypeError {
                                    expected: param_type.name().to_string(),
                                    got: arg_type.name().to_string(),
                                    context: format!("argument {} in function call", i + 1),
                                });
                            }
                        }

                        *return_type
                    }

                    Type::Any | Type::Unknown => {
                        // Unknown function type - assume valid
                        Type::Any
                    }

                    _ => {
                        self.errors.push(SemanticError::TypeError {
                            expected: "Function".to_string(),
                            got: func_type.name().to_string(),
                            context: "function call".to_string(),
                        });
                        Type::Unknown
                    }
                }
            }

            // === Data Structures ===
            AstNode::List(elements) => {
                let elem_types: Vec<Type> = elements.iter()
                    .map(|elem| self.analyze_node(elem))
                    .collect();

                // For now, assume heterogeneous lists (Type::Any)
                Type::List(Box::new(Type::Any))
            }

            AstNode::Map(fields) => {
                for (_, value) in fields {
                    self.analyze_node(value);
                }
                Type::Map
            }

            AstNode::FieldAccess { object, field } => {
                let obj_type = self.analyze_node(object);

                match obj_type {
                    Type::Map | Type::Any | Type::Unknown => Type::Any,
                    _ => {
                        self.errors.push(SemanticError::TypeError {
                            expected: "Map".to_string(),
                            got: obj_type.name().to_string(),
                            context: format!("field access .{}", field),
                        });
                        Type::Unknown
                    }
                }
            }

            AstNode::IndexAccess { object, index } => {
                let obj_type = self.analyze_node(object);
                let idx_type = self.analyze_node(index);

                match obj_type {
                    Type::List(_) => {
                        // Index must be Number
                        if !matches!(idx_type, Type::Number | Type::Any | Type::Unknown) {
                            self.errors.push(SemanticError::TypeError {
                                expected: "Number".to_string(),
                                got: idx_type.name().to_string(),
                                context: "list index".to_string(),
                            });
                        }
                        Type::Any  // Element type
                    }
                    Type::Map => {
                        // Index can be Text
                        Type::Any  // Value type
                    }
                    Type::Any | Type::Unknown => Type::Any,
                    _ => {
                        self.errors.push(SemanticError::TypeError {
                            expected: "List or Map".to_string(),
                            got: obj_type.name().to_string(),
                            context: "index access".to_string(),
                        });
                        Type::Unknown
                    }
                }
            }

            AstNode::Range { start, end } => {
                let start_type = self.analyze_node(start);
                let end_type = self.analyze_node(end);

                if !matches!(start_type, Type::Number | Type::Any | Type::Unknown) {
                    self.errors.push(SemanticError::TypeError {
                        expected: "Number".to_string(),
                        got: start_type.name().to_string(),
                        context: "range start".to_string(),
                    });
                }

                if !matches!(end_type, Type::Number | Type::Any | Type::Unknown) {
                    self.errors.push(SemanticError::TypeError {
                        expected: "Number".to_string(),
                        got: end_type.name().to_string(),
                        context: "range end".to_string(),
                    });
                }

                Type::Range
            }

            // === Expression Statement ===
            AstNode::ExprStmt(expr) => self.analyze_node(expr),

            // === Block ===
            AstNode::Block(stmts) => {
                let mut result_type = Type::Nothing;
                for stmt in stmts {
                    result_type = self.analyze_node(stmt);
                }
                result_type
            }

            // === Not Yet Implemented ===
            AstNode::MatchStmt { value, arms } => {
                use crate::ast::Pattern;

                // Analyze the value being matched
                let _match_type = self.analyze_node(value);

                // Check exhaustiveness: a match is exhaustive if it has:
                // 1. A wildcard pattern (otherwise), OR
                // 2. An identifier pattern (variable binding - matches anything)
                let has_catch_all = arms.iter().any(|arm| {
                    matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_))
                });

                if !has_catch_all {
                    self.errors.push(SemanticError::NonExhaustiveMatch {
                        message: "Match expression must have a catch-all pattern (wildcard or variable binding)".to_string(),
                    });
                }

                // Analyze each arm's body
                let mut arm_types = Vec::new();
                for arm in arms {
                    // Push new scope for pattern variables
                    self.symbol_table.push_scope();

                    // If pattern is an identifier, add it to scope
                    if let Pattern::Ident(var_name) = &arm.pattern {
                        // Bind the variable with Any type (we don't know the exact type yet)
                        let _ = self.symbol_table.define(var_name.clone(), Type::Any, false);
                    }

                    // Analyze arm body
                    for stmt in &arm.body {
                        arm_types.push(self.analyze_node(stmt));
                    }

                    // Pop scope
                    self.symbol_table.pop_scope();
                }

                // Return the type of the first arm (simplified - could unify all arm types)
                arm_types.first().cloned().unwrap_or(Type::Nothing)
            }

            AstNode::AttemptStmt { .. } => {
                // TODO: Implement error handling analysis
                Type::Any
            }

            AstNode::RequestStmt { .. } => {
                // TODO: Implement capability analysis
                Type::Capability
            }

            AstNode::Pipeline { .. } => {
                // TODO: Implement pipeline analysis
                Type::Any
            }

            AstNode::SeekExpr { .. } => {
                // TODO: Implement query analysis
                Type::Any
            }
        }
    }

    /// Convert AST TypeAnnotation to semantic Type
    fn convert_type_annotation(ann: &crate::ast::TypeAnnotation) -> Type {
        use crate::ast::TypeAnnotation;
        match ann {
            TypeAnnotation::Named(name) => match name.as_str() {
                "Number" => Type::Number,
                "Text" => Type::Text,
                "Truth" => Type::Truth,
                "Nothing" => Type::Nothing,
                "Map" => Type::Map,
                _ => Type::Unknown, // Unknown type name
            },
            TypeAnnotation::List(inner) => {
                Type::List(Box::new(Self::convert_type_annotation(inner)))
            }
            TypeAnnotation::Map => Type::Map,
            TypeAnnotation::Function { param_types, return_type } => Type::Function {
                params: param_types
                    .iter()
                    .map(|t| Self::convert_type_annotation(t))
                    .collect(),
                return_type: Box::new(Self::convert_type_annotation(return_type)),
            },
            TypeAnnotation::Optional(_) => {
                // Optional types not yet supported, treat as Any for now
                Type::Any
            }
        }
    }
}

/// Analyze a Glimmer-Weave program for semantic errors
pub fn analyze(nodes: &[AstNode]) -> Result<(), Vec<SemanticError>> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_exhaustive_match_with_wildcard() {
        // match 1 with
        //     when 1 then 100
        //     otherwise then 999
        // end
        let ast = vec![AstNode::MatchStmt {
            value: Box::new(AstNode::Number(1.0)),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(AstNode::Number(1.0)),
                    body: vec![AstNode::Number(100.0)],
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    body: vec![AstNode::Number(999.0)],
                },
            ],
        }];

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);

        // Should not have any errors - match is exhaustive
        assert!(result.is_ok(), "Expected no errors but got: {:?}", result);
    }

    #[test]
    fn test_exhaustive_match_with_variable_binding() {
        // match 42 with
        //     when n then n * 2
        // end
        let ast = vec![AstNode::MatchStmt {
            value: Box::new(AstNode::Number(42.0)),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Ident("n".to_string()),
                    body: vec![AstNode::BinaryOp {
                        left: Box::new(AstNode::Ident("n".to_string())),
                        op: BinaryOperator::Mul,
                        right: Box::new(AstNode::Number(2.0)),
                    }],
                },
            ],
        }];

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);

        // Should not have any errors - match is exhaustive
        assert!(result.is_ok(), "Expected no errors but got: {:?}", result);
    }

    #[test]
    fn test_non_exhaustive_match() {
        // match 2 with
        //     when 1 then 100
        //     when 2 then 200
        // end
        // Missing catch-all!
        let ast = vec![AstNode::MatchStmt {
            value: Box::new(AstNode::Number(2.0)),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(AstNode::Number(1.0)),
                    body: vec![AstNode::Number(100.0)],
                },
                MatchArm {
                    pattern: Pattern::Literal(AstNode::Number(2.0)),
                    body: vec![AstNode::Number(200.0)],
                },
            ],
        }];

        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);

        // Should have a non-exhaustive match error
        assert!(result.is_err(), "Expected error for non-exhaustive match");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::NonExhaustiveMatch { .. }));
    }
}
