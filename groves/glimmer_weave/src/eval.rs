//! # Evaluator Module
//!
//! Runtime execution engine for Glimmer-Weave programs.
//!
//! The evaluator interprets AST nodes and manages runtime state including:
//! - Variable bindings (immutable and mutable)
//! - Function definitions and calls
//! - Control flow (if, for, match)
//! - Error handling (attempt/harmonize)
//! - Capability requests (via kernel syscalls)

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::ast::*;

/// Runtime value types in Glimmer-Weave
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Numeric value (f64)
    Number(f64),
    /// String value
    Text(String),
    /// Boolean value
    Truth(bool),
    /// Null/void value
    Nothing,
    /// List of values
    List(Vec<Value>),
    /// Map from string keys to values
    Map(BTreeMap<String, Value>),
    /// Function (stored as AST for now - could be bytecode later)
    Chant {
        params: Vec<String>,
        body: Vec<AstNode>,
        closure: Environment,
    },
    /// Native function (builtin runtime library function)
    NativeChant(crate::runtime::NativeFunction),
    /// Capability token (unforgeable reference to kernel resource)
    Capability {
        resource: String,
        permissions: Vec<String>,
    },
    /// Range of values (for iteration)
    Range {
        start: Box<Value>,
        end: Box<Value>,
    },
    /// Outcome type - represents success (Triumph) or failure (Mishap)
    /// Similar to Rust's Result<T, E>
    Outcome {
        success: bool,  // true = Triumph, false = Mishap
        value: Box<Value>,
    },
    /// Maybe type - represents presence (Present) or absence (Absent)
    /// Similar to Rust's Option<T>
    Maybe {
        present: bool,  // true = Present, false = Absent
        value: Option<Box<Value>>,
    },
    /// Struct definition - represents a struct type
    StructDef {
        name: String,
        fields: Vec<crate::ast::StructField>,
    },
    /// Struct instance - represents an instance of a struct
    StructInstance {
        struct_name: String,
        fields: BTreeMap<String, Value>,
    },
}

impl Value {
    /// Check if value is truthy (for conditionals)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Truth(b) => *b,
            Value::Nothing => false,
            Value::Number(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            _ => true,
        }
    }

    /// Convert to human-readable string (for debugging)
    pub fn type_name(&self) -> &str {
        match self {
            Value::Number(_) => "Number",
            Value::Text(_) => "Text",
            Value::Truth(_) => "Truth",
            Value::Nothing => "Nothing",
            Value::List(_) => "List",
            Value::Map(_) => "Map",
            Value::Chant { .. } => "Chant",
            Value::NativeChant(_) => "NativeChant",
            Value::Capability { .. } => "Capability",
            Value::Range { .. } => "Range",
            Value::Outcome { success, .. } => if *success { "Triumph" } else { "Mishap" },
            Value::Maybe { present, .. } => if *present { "Present" } else { "Absent" },
            Value::StructDef { name, .. } => return name.as_str(),
            Value::StructInstance { struct_name, .. } => return struct_name.as_str(),
        }
    }
}

/// Runtime errors that can occur during evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// Variable not found in scope
    UndefinedVariable(String),
    /// Attempt to mutate immutable binding
    ImmutableBinding(String),
    /// Type mismatch (expected X, got Y)
    TypeError {
        expected: String,
        got: String,
    },
    /// Division by zero
    DivisionByZero,
    /// Index out of bounds
    IndexOutOfBounds {
        index: usize,
        length: usize,
    },
    /// Field not found on map
    FieldNotFound {
        field: String,
        object: String,
    },
    /// Value is not iterable (for loops)
    NotIterable(String),
    /// Value is not callable (function calls)
    NotCallable(String),
    /// Wrong number of arguments
    ArityMismatch {
        expected: usize,
        got: usize,
    },
    /// Capability request denied
    CapabilityDenied {
        capability: String,
        reason: String,
    },
    /// Return statement outside of function
    UnexpectedYield,
    /// Pattern match failed (no arm matched)
    MatchFailed,
    /// Early return from function (not actually an error, used for control flow)
    Return(Value),
    /// Tail call continuation (for TCO - not an error, used for control flow)
    TailCall {
        function_name: String,
        args: Vec<Value>,
    },
    /// Custom error message
    Custom(String),
    /// Bytecode compilation error
    CompileError {
        message: String,
    },
}

impl RuntimeError {
    /// Get the error type name for error handling
    pub fn error_type(&self) -> &str {
        match self {
            RuntimeError::UndefinedVariable(_) => "UndefinedVariable",
            RuntimeError::ImmutableBinding(_) => "ImmutableBinding",
            RuntimeError::TypeError { .. } => "TypeError",
            RuntimeError::DivisionByZero => "DivisionByZero",
            RuntimeError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
            RuntimeError::FieldNotFound { .. } => "FieldNotFound",
            RuntimeError::NotIterable(_) => "NotIterable",
            RuntimeError::NotCallable(_) => "NotCallable",
            RuntimeError::ArityMismatch { .. } => "ArityMismatch",
            RuntimeError::CapabilityDenied { .. } => "CapabilityDenied",
            RuntimeError::UnexpectedYield => "UnexpectedYield",
            RuntimeError::MatchFailed => "MatchFailed",
            RuntimeError::Return(_) => "Return",
            RuntimeError::TailCall { .. } => "TailCall",
            RuntimeError::Custom(_) => "CustomError",
            RuntimeError::CompileError { .. } => "CompileError",
        }
    }

    /// Get the error value for binding in error handlers
    pub fn error_value(&self) -> Value {
        match self {
            RuntimeError::Custom(msg) => Value::Text(msg.clone()),
            RuntimeError::UndefinedVariable(name) => Value::Text(name.clone()),
            RuntimeError::ImmutableBinding(name) => Value::Text(name.clone()),
            RuntimeError::TypeError { expected, got } => {
                Value::Text(format!("Expected {}, got {}", expected, got))
            }
            RuntimeError::DivisionByZero => Value::Text("Division by zero".to_string()),
            RuntimeError::IndexOutOfBounds { index, length } => {
                Value::Text(format!("Index {} out of bounds (length {})", index, length))
            }
            RuntimeError::FieldNotFound { field, object } => {
                Value::Text(format!("Field '{}' not found on {}", field, object))
            }
            RuntimeError::NotIterable(t) => Value::Text(format!("{} is not iterable", t)),
            RuntimeError::NotCallable(t) => Value::Text(format!("{} is not callable", t)),
            RuntimeError::ArityMismatch { expected, got } => {
                Value::Text(format!("Expected {} arguments, got {}", expected, got))
            }
            RuntimeError::CapabilityDenied { capability, reason } => {
                Value::Text(format!("Capability '{}' denied: {}", capability, reason))
            }
            RuntimeError::UnexpectedYield => Value::Text("Unexpected yield outside function".to_string()),
            RuntimeError::MatchFailed => Value::Text("No pattern matched".to_string()),
            RuntimeError::CompileError { message } => Value::Text(message.clone()),
            RuntimeError::Return(val) => val.clone(),
            RuntimeError::TailCall { function_name, .. } => Value::Text(format!("Tail call to {}", function_name)),
        }
    }
}

/// Variable binding with mutability tracking
#[derive(Debug, Clone, PartialEq)]
struct Binding {
    value: Value,
    mutable: bool,
}

/// Environment manages variable scopes
///
/// Scopes are nested: inner scopes can shadow outer scopes.
/// When a function is called, we push a new scope.
/// When it returns, we pop the scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    /// Stack of scopes (innermost scope is last)
    scopes: Vec<BTreeMap<String, Binding>>,
}

impl Environment {
    /// Create a new environment with one empty scope
    pub fn new() -> Self {
        Environment {
            scopes: alloc::vec![BTreeMap::new()],
        }
    }

    /// Push a new scope (for function calls, blocks)
    pub fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// Pop the innermost scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a new immutable binding
    pub fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Binding { value, mutable: false });
        }
    }

    /// Define a new mutable binding
    pub fn define_mut(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Binding { value, mutable: true });
        }
    }

    /// Get a variable's value (searches from innermost to outermost scope)
    pub fn get(&self, name: &str) -> Result<Value, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Ok(binding.value.clone());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }

    /// Set a variable's value (must be mutable)
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(RuntimeError::ImmutableBinding(name.to_string()));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(RuntimeError::UndefinedVariable(name.to_string()))
    }
}

/// Evaluator executes Glimmer-Weave programs
pub struct Evaluator {
    environment: Environment,
}

impl Evaluator {
    /// Create a new evaluator with empty environment
    pub fn new() -> Self {
        let mut evaluator = Evaluator {
            environment: Environment::new(),
        };

        // Register builtin runtime library functions
        for builtin in crate::runtime::get_builtins() {
            evaluator.environment.define(
                builtin.name.clone(),
                Value::NativeChant(builtin),
            );
        }

        evaluator
    }

    /// Evaluate a list of statements (program or block)
    pub fn eval(&mut self, nodes: &[AstNode]) -> Result<Value, RuntimeError> {
        let mut result = Value::Nothing;
        for node in nodes {
            result = self.eval_node(node)?;
        }
        Ok(result)
    }

    /// Evaluate using the bytecode VM (Quicksilver fast path)
    ///
    /// This provides 5-10x performance improvement for pure expressions
    /// that don't require stateful environment access.
    ///
    /// **Limitations:**
    /// - Does not support local variables (only globals)
    /// - Does not support function calls (chants)
    /// - Does not support capability requests
    ///
    /// **Use cases:**
    /// - Arithmetic expressions
    /// - Comparisons
    /// - Simple data structures (lists, maps)
    /// - Global variable access
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of execution
    /// * `Err(RuntimeError)` - If compilation or execution fails
    pub fn eval_with_vm(&mut self, nodes: &[AstNode]) -> Result<Value, RuntimeError> {
        use crate::bytecode_compiler::compile;
        use crate::vm::{VM, VmError};

        // Compile AST to bytecode
        let chunk = compile(nodes).map_err(|e| RuntimeError::CompileError {
            message: alloc::format!("{:?}", e),
        })?;

        // Execute in VM
        let mut vm = VM::new();
        let result = vm.execute(chunk).map_err(|e| match e {
            VmError::TypeError(msg) => RuntimeError::TypeError {
                expected: "compatible type".to_string(),
                got: msg,
            },
            VmError::UndefinedVariable(name) => RuntimeError::UndefinedVariable(name),
            VmError::DivisionByZero => RuntimeError::DivisionByZero,
            VmError::OutOfBounds => RuntimeError::IndexOutOfBounds {
                index: 0,
                length: 0,
            },
            _ => RuntimeError::CompileError {
                message: alloc::format!("{:?}", e),
            },
        })?;

        Ok(result)
    }

    /// Evaluate a single AST node
    pub fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
        match node {
            // === Literals ===
            AstNode::Number(n) => Ok(Value::Number(*n)),
            AstNode::Text(s) => Ok(Value::Text(s.clone())),
            AstNode::Truth(b) => Ok(Value::Truth(*b)),
            AstNode::Nothing => Ok(Value::Nothing),

            // === Outcome constructors ===
            AstNode::Triumph(value) => {
                let inner = self.eval_node(value)?;
                Ok(Value::Outcome {
                    success: true,
                    value: Box::new(inner),
                })
            }

            AstNode::Mishap(value) => {
                let inner = self.eval_node(value)?;
                Ok(Value::Outcome {
                    success: false,
                    value: Box::new(inner),
                })
            }

            // === Maybe constructors ===
            AstNode::Present(value) => {
                let inner = self.eval_node(value)?;
                Ok(Value::Maybe {
                    present: true,
                    value: Some(Box::new(inner)),
                })
            }

            AstNode::Absent => Ok(Value::Maybe {
                present: false,
                value: None,
            }),

            // === Variables ===
            AstNode::Ident(name) => self.environment.get(name),

            // === Lists ===
            AstNode::List(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.eval_node(elem)?);
                }
                Ok(Value::List(values))
            }

            // === Maps ===
            AstNode::Map(pairs) => {
                let mut map = BTreeMap::new();
                for (key, value_node) in pairs {
                    let value = self.eval_node(value_node)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Map(map))
            }

            // === Statements ===

            // bind x to 42
            AstNode::BindStmt { name, typ: _, value } => {
                // Type annotations are checked by semantic analyzer, ignored at runtime
                let val = self.eval_node(value)?;
                self.environment.define(name.clone(), val.clone());
                Ok(val)
            }

            // weave counter as 0
            AstNode::WeaveStmt { name, typ: _, value } => {
                // Type annotations are checked by semantic analyzer, ignored at runtime
                let val = self.eval_node(value)?;
                self.environment.define_mut(name.clone(), val.clone());
                Ok(val)
            }

            // set counter to 10
            AstNode::SetStmt { name, value } => {
                let val = self.eval_node(value)?;
                self.environment.set(name, val.clone())?;
                Ok(val)
            }

            // should condition then ... otherwise ... end
            AstNode::IfStmt { condition, then_branch, else_branch } => {
                let cond_val = self.eval_node(condition)?;
                if cond_val.is_truthy() {
                    self.eval(then_branch)
                } else if let Some(else_body) = else_branch {
                    self.eval(else_body)
                } else {
                    Ok(Value::Nothing)
                }
            }

            // for each x in list then ... end
            AstNode::ForStmt { variable, iterable, body } => {
                let iter_val = self.eval_node(iterable)?;

                let items = match iter_val {
                    Value::List(ref items) => items.clone(),
                    Value::Range { start, end } => {
                        // Generate range values
                        let mut items = Vec::new();
                        let start_num = match start.as_ref() {
                            Value::Number(n) => *n as i64,
                            _ => return Err(RuntimeError::TypeError {
                                expected: "Number".to_string(),
                                got: start.type_name().to_string(),
                            }),
                        };
                        let end_num = match end.as_ref() {
                            Value::Number(n) => *n as i64,
                            _ => return Err(RuntimeError::TypeError {
                                expected: "Number".to_string(),
                                got: end.type_name().to_string(),
                            }),
                        };
                        for i in start_num..end_num {
                            items.push(Value::Number(i as f64));
                        }
                        items
                    }
                    _ => return Err(RuntimeError::NotIterable(iter_val.type_name().to_string())),
                };

                let mut result = Value::Nothing;
                for item in items {
                    self.environment.push_scope();
                    self.environment.define(variable.clone(), item);
                    result = self.eval(body)?;
                    self.environment.pop_scope();
                }
                Ok(result)
            }

            // whilst condition then ... end
            AstNode::WhileStmt { condition, body } => {
                let mut result = Value::Nothing;
                loop {
                    let cond_val = self.eval_node(condition)?;
                    if !cond_val.is_truthy() {
                        break;
                    }
                    result = self.eval(body)?;
                }
                Ok(result)
            }

            // chant greet(name) then ... end
            AstNode::ChantDef { name, params, return_type: _, body } => {
                // Type annotations are checked by semantic analyzer, extract param names only
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Clone environment and add function to it for recursion support
                let mut closure_env = self.environment.clone();

                // Create the function value
                let chant = Value::Chant {
                    params: param_names.clone(),
                    body: body.clone(),
                    closure: closure_env.clone(),
                };

                // Add function to its own closure so it can call itself recursively
                closure_env.define(name.clone(), chant.clone());

                // Update the closure to include the function itself
                let chant = Value::Chant {
                    params: param_names,
                    body: body.clone(),
                    closure: closure_env,
                };

                // Define in current environment
                self.environment.define(name.clone(), chant.clone());
                Ok(chant)
            }

            AstNode::FormDef { name, fields } => {
                // Create struct definition
                let struct_def = Value::StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                };

                // Define in current environment
                self.environment.define(name.clone(), struct_def.clone());
                Ok(struct_def)
            }

            AstNode::StructLiteral { struct_name, fields: field_values } => {
                // Look up the struct definition
                let struct_def = self.environment.get(struct_name)?;

                match struct_def {
                    Value::StructDef { name: _, fields } => {
                        // Evaluate all field values
                        let mut evaluated_fields = BTreeMap::new();
                        for (field_name, field_expr) in field_values {
                            let value = self.eval_node(field_expr)?;
                            evaluated_fields.insert(field_name.clone(), value);
                        }

                        // Check that all required fields are provided
                        for field in &fields {
                            if !evaluated_fields.contains_key(&field.name) {
                                return Err(RuntimeError::Custom(
                                    format!("Missing field '{}' in struct '{}'", field.name, struct_name)
                                ));
                            }
                        }

                        // Create struct instance
                        Ok(Value::StructInstance {
                            struct_name: struct_name.clone(),
                            fields: evaluated_fields,
                        })
                    }
                    _ => Err(RuntimeError::TypeError {
                        expected: "struct definition".to_string(),
                        got: struct_def.type_name().to_string(),
                    }),
                }
            }

            // yield result
            AstNode::YieldStmt { value } => {
                // Check if we're yielding a call (potential tail call)
                if let AstNode::Call { callee, args } = value.as_ref() {
                    // Check if callee is an identifier
                    if let AstNode::Ident(func_name) = callee.as_ref() {
                        // Check if it's a tail call to the current function
                        if let Ok(Value::Text(current_func)) = self.environment.get("__current_function__") {
                            if func_name == &current_func {
                                // This is a tail-recursive call!
                                // Evaluate args and throw TailCall instead of Return
                                let arg_vals: Result<Vec<Value>, RuntimeError> =
                                    args.iter().map(|arg| self.eval_node(arg)).collect();
                                let arg_vals = arg_vals?;

                                return Err(RuntimeError::TailCall {
                                    function_name: func_name.clone(),
                                    args: arg_vals,
                                });
                            }
                        }
                    }
                }

                // Not a tail call, evaluate normally
                let val = self.eval_node(value)?;
                Err(RuntimeError::Return(val))
            }

            // === Binary Operations ===
            AstNode::BinaryOp { left, op, right } => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }

            // === Unary Operations ===
            AstNode::UnaryOp { op, operand } => {
                let val = self.eval_node(operand)?;
                self.eval_unary_op(*op, &val)
            }

            // === Function Calls ===
            AstNode::Call { callee, args } => {
                let func = self.eval_node(callee)?;
                let arg_vals: Result<Vec<Value>, RuntimeError> =
                    args.iter().map(|arg| self.eval_node(arg)).collect();
                let arg_vals = arg_vals?;

                match func {
                    Value::Chant { params, body, closure: _ } => {
                        if params.len() != arg_vals.len() {
                            return Err(RuntimeError::ArityMismatch {
                                expected: params.len(),
                                got: arg_vals.len(),
                            });
                        }

                        // Get function name if callee is an Ident (for TCO detection)
                        let func_name = match callee.as_ref() {
                            AstNode::Ident(name) => Some(name.clone()),
                            _ => None,
                        };

                        // Trampoline loop for TCO
                        let mut current_args = arg_vals;
                        loop {
                            // Push new scope for function call
                            self.environment.push_scope();

                            // Bind parameters
                            for (param, arg) in params.iter().zip(current_args.iter()) {
                                self.environment.define(param.clone(), arg.clone());
                            }

                            // Store function name for tail call detection
                            if let Some(ref name) = func_name {
                                self.environment.define("__current_function__".to_string(), Value::Text(name.clone()));
                            }

                            // Execute function body
                            let result = self.eval(&body);

                            // Restore environment
                            self.environment.pop_scope();

                            // Handle result
                            match result {
                                Err(RuntimeError::Return(val)) => return Ok(val),
                                Err(RuntimeError::TailCall { function_name, args }) => {
                                    // Check if it's a recursive tail call
                                    if Some(&function_name) == func_name.as_ref() {
                                        // TCO: Loop with new args instead of recursing!
                                        current_args = args;
                                        continue;
                                    } else {
                                        // Not a recursive call, re-throw to propagate up
                                        return Err(RuntimeError::TailCall { function_name, args });
                                    }
                                }
                                other => return other,
                            }
                        }
                    }
                    Value::NativeChant(native_fn) => {
                        // Check arity (None = variadic)
                        if let Some(expected) = native_fn.arity {
                            if arg_vals.len() != expected {
                                return Err(RuntimeError::ArityMismatch {
                                    expected,
                                    got: arg_vals.len(),
                                });
                            }
                        }

                        // Call native function
                        (native_fn.func)(&arg_vals)
                    }
                    _ => Err(RuntimeError::NotCallable(func.type_name().to_string())),
                }
            }

            // === Field Access ===
            AstNode::FieldAccess { object, field } => {
                let obj = self.eval_node(object)?;
                match obj {
                    Value::Map(ref map) => {
                        map.get(field)
                            .cloned()
                            .ok_or_else(|| RuntimeError::FieldNotFound {
                                field: field.clone(),
                                object: "Map".to_string(),
                            })
                    }
                    Value::StructInstance { struct_name, ref fields } => {
                        fields.get(field)
                            .cloned()
                            .ok_or_else(|| RuntimeError::FieldNotFound {
                                field: field.clone(),
                                object: struct_name.clone(),
                            })
                    }
                    _ => Err(RuntimeError::TypeError {
                        expected: "Map or Struct".to_string(),
                        got: obj.type_name().to_string(),
                    }),
                }
            }

            // === Index Access ===
            AstNode::IndexAccess { object, index } => {
                let obj = self.eval_node(object)?;
                let idx = self.eval_node(index)?;

                match (obj, idx) {
                    (Value::List(ref list), Value::Number(n)) => {
                        let index = n as usize;
                        if index < list.len() {
                            Ok(list[index].clone())
                        } else {
                            Err(RuntimeError::IndexOutOfBounds {
                                index,
                                length: list.len(),
                            })
                        }
                    }
                    (Value::Map(ref map), Value::Text(key)) => {
                        map.get(&key)
                            .cloned()
                            .ok_or_else(|| RuntimeError::FieldNotFound {
                                field: key,
                                object: "Map".to_string(),
                            })
                    }
                    (obj, idx) => Err(RuntimeError::TypeError {
                        expected: "List or Map".to_string(),
                        got: alloc::format!("{} with {} index", obj.type_name(), idx.type_name()),
                    }),
                }
            }

            // === Range ===
            AstNode::Range { start, end } => {
                let start_val = self.eval_node(start)?;
                let end_val = self.eval_node(end)?;
                Ok(Value::Range {
                    start: Box::new(start_val),
                    end: Box::new(end_val),
                })
            }

            // === Expression Statement ===
            AstNode::ExprStmt(expr) => self.eval_node(expr),

            // === Block ===
            AstNode::Block(statements) => {
                self.environment.push_scope();
                let result = self.eval(statements);
                self.environment.pop_scope();
                result
            }

            // === Pattern Matching ===
            AstNode::MatchStmt { value, arms } => {
                use crate::ast::Pattern;

                // Evaluate the value to match against
                let match_value = self.eval_node(value)?;

                // Try each arm in order
                for arm in arms {
                    // Check if pattern matches
                    if let Some(bindings) = self.pattern_matches(&arm.pattern, &match_value)? {
                        // Pattern matched! Create new scope and bind variables
                        self.environment.push_scope();

                        // Bind pattern variables
                        for (name, val) in bindings {
                            self.environment.define(name, val);
                        }

                        // Execute the arm body
                        let mut result = Value::Nothing;
                        for stmt in &arm.body {
                            result = self.eval_node(stmt)?;
                        }

                        // Pop scope and return result
                        self.environment.pop_scope();
                        return Ok(result);
                    }
                }

                // No pattern matched
                Err(RuntimeError::Custom("No pattern matched".to_string()))
            }
            AstNode::AttemptStmt { body, handlers } => {
                // Try to execute the body
                let result = self.eval(body);

                // If successful, return the result
                if result.is_ok() {
                    return result;
                }

                // An error occurred - try to find a matching handler
                let error = result.unwrap_err();

                // Don't catch Return or TailCall - these are control flow, not errors
                if matches!(error, RuntimeError::Return(_) | RuntimeError::TailCall { .. }) {
                    return Err(error);
                }

                // Get the error type for matching
                let error_type = error.error_type();

                // Try to find a matching handler
                for handler in handlers {
                    // Check if this handler matches the error type
                    // Support wildcard "_" to catch all errors
                    if handler.error_type == error_type || handler.error_type == "_" {
                        // Execute the handler body
                        return self.eval(&handler.body);
                    }
                }

                // No handler matched - propagate the error
                Err(error)
            }
            AstNode::RequestStmt { .. } => {
                Err(RuntimeError::Custom("Capability requests not yet implemented".to_string()))
            }
            AstNode::Pipeline { .. } => {
                Err(RuntimeError::Custom("Pipelines not yet implemented".to_string()))
            }
            AstNode::SeekExpr { .. } => {
                Err(RuntimeError::Custom("World-Tree queries not yet implemented".to_string()))
            }
        }
    }

    /// Check if a pattern matches a value, returning bindings if it matches
    fn pattern_matches(
        &mut self,
        pattern: &crate::ast::Pattern,
        value: &Value,
    ) -> Result<Option<Vec<(String, Value)>>, RuntimeError> {
        use crate::ast::Pattern;

        match pattern {
            // Literal patterns - must match exactly
            Pattern::Literal(lit_node) => {
                let lit_value = self.eval_node(lit_node)?;
                if &lit_value == value {
                    Ok(Some(Vec::new())) // Match with no bindings
                } else {
                    Ok(None) // No match
                }
            }

            // Variable binding pattern - matches anything and binds
            Pattern::Ident(name) => {
                Ok(Some(vec![(name.clone(), value.clone())]))
            }

            // Wildcard pattern - matches anything without binding
            Pattern::Wildcard => {
                Ok(Some(Vec::new()))
            }

            // Enum pattern - matches Outcome or Maybe variants
            Pattern::Enum { variant, inner } => {
                match (variant.as_str(), value) {
                    // Match Triumph(x)
                    ("Triumph", Value::Outcome { success: true, value: inner_value }) => {
                        if let Some(inner_pattern) = inner {
                            // Recursively match the inner pattern
                            self.pattern_matches(inner_pattern, inner_value)
                        } else {
                            Ok(Some(Vec::new()))
                        }
                    }

                    // Match Mishap(e)
                    ("Mishap", Value::Outcome { success: false, value: inner_value }) => {
                        if let Some(inner_pattern) = inner {
                            self.pattern_matches(inner_pattern, inner_value)
                        } else {
                            Ok(Some(Vec::new()))
                        }
                    }

                    // Match Present(v)
                    ("Present", Value::Maybe { present: true, value: Some(inner_value) }) => {
                        if let Some(inner_pattern) = inner {
                            self.pattern_matches(inner_pattern, inner_value)
                        } else {
                            Ok(Some(Vec::new()))
                        }
                    }

                    // Match Absent
                    ("Absent", Value::Maybe { present: false, value: None }) => {
                        if inner.is_none() {
                            Ok(Some(Vec::new()))
                        } else {
                            Ok(None) // Absent shouldn't have inner pattern
                        }
                    }

                    // No match
                    _ => Ok(None),
                }
            }
        }
    }

    /// Evaluate binary operation
    fn eval_binary_op(
        &self,
        left: &Value,
        op: BinaryOperator,
        right: &Value,
    ) -> Result<Value, RuntimeError> {
        match (left, op, right) {
            // Arithmetic
            (Value::Number(l), BinaryOperator::Add, Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Number(l), BinaryOperator::Sub, Value::Number(r)) => Ok(Value::Number(l - r)),
            (Value::Number(l), BinaryOperator::Mul, Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Number(l), BinaryOperator::Div, Value::Number(r)) => {
                if *r == 0.0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Number(l / r))
                }
            }
            (Value::Number(l), BinaryOperator::Mod, Value::Number(r)) => {
                if *r == 0.0 {
                    Err(RuntimeError::DivisionByZero)
                } else {
                    Ok(Value::Number(l % r))
                }
            }

            // String concatenation
            (Value::Text(l), BinaryOperator::Add, Value::Text(r)) => {
                let mut result = l.clone();
                result.push_str(r);
                Ok(Value::Text(result))
            }

            // Comparison
            (Value::Number(l), BinaryOperator::Greater, Value::Number(r)) => Ok(Value::Truth(l > r)),
            (Value::Number(l), BinaryOperator::Less, Value::Number(r)) => Ok(Value::Truth(l < r)),
            (Value::Number(l), BinaryOperator::GreaterEq, Value::Number(r)) => Ok(Value::Truth(l >= r)),
            (Value::Number(l), BinaryOperator::LessEq, Value::Number(r)) => Ok(Value::Truth(l <= r)),

            // Equality (works for all types)
            (l, BinaryOperator::Equal, r) => Ok(Value::Truth(l == r)),
            (l, BinaryOperator::NotEqual, r) => Ok(Value::Truth(l != r)),

            // Logical
            (l, BinaryOperator::And, r) => Ok(Value::Truth(l.is_truthy() && r.is_truthy())),
            (l, BinaryOperator::Or, r) => Ok(Value::Truth(l.is_truthy() || r.is_truthy())),

            // Type mismatch
            _ => Err(RuntimeError::TypeError {
                expected: left.type_name().to_string(),
                got: right.type_name().to_string(),
            }),
        }
    }

    /// Evaluate unary operation
    fn eval_unary_op(&self, op: UnaryOperator, operand: &Value) -> Result<Value, RuntimeError> {
        match (op, operand) {
            (UnaryOperator::Not, val) => Ok(Value::Truth(!val.is_truthy())),
            (UnaryOperator::Negate, Value::Number(n)) => Ok(Value::Number(-n)),
            (UnaryOperator::Negate, val) => Err(RuntimeError::TypeError {
                expected: "Number".to_string(),
                got: val.type_name().to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval_program(source: &str) -> Result<Value, RuntimeError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse error");
        let mut evaluator = Evaluator::new();
        evaluator.eval(&ast)
    }

    fn eval_with_vm_helper(source: &str) -> Result<Value, RuntimeError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parse error");
        let mut evaluator = Evaluator::new();
        evaluator.eval_with_vm(&ast)
    }

    #[test]
    fn test_vm_integration_arithmetic() {
        // Test that VM produces same results as tree-walking interpreter
        let source = "10 + 20 * 2";

        let interpreter_result = eval_program(source).expect("Interpreter failed");
        let vm_result = eval_with_vm_helper(source).expect("VM failed");

        assert_eq!(interpreter_result, vm_result);
        assert_eq!(vm_result, Value::Number(50.0));
    }

    #[test]
    fn test_vm_integration_comparison() {
        let source = "10 less than 20";

        let interpreter_result = eval_program(source).expect("Interpreter failed");
        let vm_result = eval_with_vm_helper(source).expect("VM failed");

        assert_eq!(interpreter_result, vm_result);
        assert_eq!(vm_result, Value::Truth(true));
    }

    #[test]
    fn test_vm_integration_global_variables() {
        let source = "bind x to 42\nx + 8";

        let interpreter_result = eval_program(source).expect("Interpreter failed");
        let vm_result = eval_with_vm_helper(source).expect("VM failed");

        assert_eq!(interpreter_result, vm_result);
        assert_eq!(vm_result, Value::Number(50.0));
    }

    #[test]
    fn test_while_loop_countdown() {
        let source = r#"
weave counter as 5
weave sum as 0

whilst counter greater than 0 then
    set sum to sum + counter
    set counter to counter - 1
end

sum
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(15.0)); // 5+4+3+2+1 = 15
    }

    #[test]
    fn test_while_loop_with_break_condition() {
        let source = r#"
weave x as 0
whilst x less than 100 then
    set x to x + 1
end
x
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(100.0));
    }

    #[test]
    fn test_factorial_via_recursion() {
        let source = r#"
chant factorial(n) then
    should n at most 1 then
        yield 1
    otherwise
        yield n * factorial(n - 1)
    end
end

factorial(5)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(120.0)); // 5! = 120
    }

    #[test]
    fn test_fibonacci_via_while_loop() {
        let source = r#"
chant fibonacci(n) then
    should n at most 1 then
        yield n
    end

    weave a as 0
    weave b as 1
    weave count as 2

    whilst count at most n then
        weave temp as a + b
        set a to b
        set b to temp
        set count to count + 1
    end

    yield b
end

fibonacci(10)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(55.0)); // 10th Fibonacci number
    }

    #[test]
    fn test_nested_while_loops() {
        let source = r#"
weave sum as 0
weave i as 1

whilst i at most 3 then
    weave j as 1
    whilst j at most 3 then
        set sum to sum + 1
        set j to j + 1
    end
    set i to i + 1
end

sum
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(9.0)); // 3x3 = 9
    }

    #[test]
    fn test_recursion_with_accumulator() {
        // This is tail-recursive and uses TCO (no stack overflow!)
        let source = r#"
chant sum_to(n, acc) then
    should n at most 0 then
        yield acc
    otherwise
        yield sum_to(n - 1, acc + n)
    end
end

sum_to(100, 0)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(5050.0)); // Sum of 1..100 = 5050
    }

    #[test]
    fn test_turing_completeness_collatz() {
        // The Collatz conjecture test - unbounded iteration
        let source = r#"
chant collatz_steps(n) then
    weave steps as 0
    weave num as n

    whilst num greater than 1 then
        should num % 2 is 0 then
            set num to num / 2
        otherwise
            set num to 3 * num + 1
        end
        set steps to steps + 1
    end

    yield steps
end

collatz_steps(27)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(111.0)); // Collatz(27) takes 111 steps
    }

    #[test]
    fn test_pattern_matching_literals() {
        let source = r#"
bind x to 2

match x with
    when 1 then "one"
    when 2 then "two"
    when 3 then "three"
    otherwise then "other"
end
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("two".to_string()));
    }

    #[test]
    fn test_pattern_matching_variable_binding() {
        let source = r#"
bind x to 42

match x with
    when 0 then "zero"
    when n then n * 2
end
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(84.0));
    }

    #[test]
    fn test_pattern_matching_wildcard() {
        let source = r#"
bind x to 99

match x with
    when 1 then "one"
    when 2 then "two"
    otherwise then "something else"
end
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("something else".to_string()));
    }

    #[test]
    fn test_pattern_matching_fizzbuzz() {
        // Pattern matching makes FizzBuzz elegant
        let source = r#"
chant fizzbuzz(n) then
    bind mod15 to n % 15
    bind mod5 to n % 5
    bind mod3 to n % 3

    match mod15 with
        when 0 then yield "FizzBuzz"
        otherwise then
            match mod5 with
                when 0 then yield "Buzz"
                otherwise then
                    match mod3 with
                        when 0 then yield "Fizz"
                        otherwise then yield n
                    end
            end
    end
end

fizzbuzz(15)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("FizzBuzz".to_string()));
    }

    #[test]
    fn test_struct_definition() {
        let source = r#"
form Person with
    name as Text
    age as Number
end

Person
        "#;

        let result = eval_program(source).expect("Eval failed");
        // Should return the struct definition itself
        match result {
            Value::StructDef { name, fields } => {
                assert_eq!(name, "Person");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "name");
                assert_eq!(fields[1].name, "age");
            }
            _ => panic!("Expected StructDef, got {:?}", result),
        }
    }

    #[test]
    fn test_struct_instantiation() {
        let source = r#"
form Person with
    name as Text
    age as Number
end

bind alice to Person { name: "Alice", age: 30 }
alice
        "#;

        let result = eval_program(source).expect("Eval failed");
        match result {
            Value::StructInstance { struct_name, fields } => {
                assert_eq!(struct_name, "Person");
                assert_eq!(fields.get("name"), Some(&Value::Text("Alice".to_string())));
                assert_eq!(fields.get("age"), Some(&Value::Number(30.0)));
            }
            _ => panic!("Expected StructInstance, got {:?}", result),
        }
    }

    #[test]
    fn test_struct_field_access() {
        let source = r#"
form Person with
    name as Text
    age as Number
end

bind alice to Person { name: "Alice", age: 30 }
alice.name
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("Alice".to_string()));
    }

    #[test]
    fn test_struct_field_access_number() {
        let source = r#"
form Person with
    name as Text
    age as Number
end

bind bob to Person { name: "Bob", age: 42 }
bob.age
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(42.0));
    }

    #[test]
    fn test_struct_in_function() {
        let source = r#"
form Point with
    x as Number
    y as Number
end

chant distance(p) then
    yield (p.x * p.x + p.y * p.y)
end

bind origin to Point { x: 3, y: 4 }
distance(origin)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(25.0)); // 3^2 + 4^2 = 9 + 16 = 25
    }

    #[test]
    fn test_nested_structs() {
        let source = r#"
form Address with
    street as Text
    city as Text
end

form Person with
    name as Text
    address as Address
end

bind addr to Address { street: "Main St", city: "Boston" }
bind alice to Person { name: "Alice", address: addr }
alice.address.city
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("Boston".to_string()));
    }

    #[test]
    fn test_struct_multiple_instances() {
        let source = r#"
form Person with
    name as Text
    age as Number
end

bind alice to Person { name: "Alice", age: 30 }
bind bob to Person { name: "Bob", age: 42 }
bob.name
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Text("Bob".to_string()));
    }

    #[test]
    fn test_struct_empty() {
        // Test struct with no fields
        let source = r#"
form Empty with
end

bind e to Empty {}
e
        "#;

        let result = eval_program(source).expect("Eval failed");
        match result {
            Value::StructInstance { struct_name, fields } => {
                assert_eq!(struct_name, "Empty");
                assert_eq!(fields.len(), 0);
            }
            _ => panic!("Expected StructInstance, got {:?}", result),
        }
    }
}
