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
    /// Custom error message
    Custom(String),
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

    /// Evaluate a single AST node
    pub fn eval_node(&mut self, node: &AstNode) -> Result<Value, RuntimeError> {
        match node {
            // === Literals ===
            AstNode::Number(n) => Ok(Value::Number(*n)),
            AstNode::Text(s) => Ok(Value::Text(s.clone())),
            AstNode::Truth(b) => Ok(Value::Truth(*b)),
            AstNode::Nothing => Ok(Value::Nothing),

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
            AstNode::BindStmt { name, value } => {
                let val = self.eval_node(value)?;
                self.environment.define(name.clone(), val.clone());
                Ok(val)
            }

            // weave counter as 0
            AstNode::WeaveStmt { name, value } => {
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
            AstNode::ChantDef { name, params, body } => {
                let chant = Value::Chant {
                    params: params.clone(),
                    body: body.clone(),
                    closure: self.environment.clone(),
                };
                self.environment.define(name.clone(), chant.clone());
                Ok(chant)
            }

            // yield result
            AstNode::YieldStmt { value } => {
                // TODO: Implement proper return mechanism
                // For now, just evaluate and return (needs special handling in function calls)
                self.eval_node(value)
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
                    Value::Chant { params, body, closure } => {
                        if params.len() != arg_vals.len() {
                            return Err(RuntimeError::ArityMismatch {
                                expected: params.len(),
                                got: arg_vals.len(),
                            });
                        }

                        // Save current environment and switch to closure
                        let saved_env = core::mem::replace(&mut self.environment, closure);

                        // Push new scope for function call
                        self.environment.push_scope();

                        // Bind parameters
                        for (param, arg) in params.iter().zip(arg_vals.iter()) {
                            self.environment.define(param.clone(), arg.clone());
                        }

                        // Execute function body
                        let result = self.eval(&body);

                        // Restore environment
                        self.environment.pop_scope();
                        self.environment = saved_env;

                        result
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
                    _ => Err(RuntimeError::TypeError {
                        expected: "Map".to_string(),
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

            // === Not Yet Implemented ===
            AstNode::MatchStmt { .. } => {
                Err(RuntimeError::Custom("Pattern matching not yet implemented".to_string()))
            }
            AstNode::AttemptStmt { .. } => {
                Err(RuntimeError::Custom("Error handling not yet implemented".to_string()))
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

    #[test]
    fn test_while_loop_countdown() {
        let source = r#"
weave counter as 5
weave sum as 0

whilst counter > 0 then
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
whilst x < 100 then
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
    should n <= 1 then
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
    should n <= 1 then
        yield n
    end

    weave a as 0
    weave b as 1
    weave count as 2

    whilst count <= n then
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

whilst i <= 3 then
    weave j as 1
    whilst j <= 3 then
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
        let source = r#"
chant sum_to(n, acc) then
    should n <= 0 then
        yield acc
    otherwise
        yield sum_to(n - 1, acc + n)
    end
end

sum_to(100, 0)
        "#;

        let result = eval_program(source).expect("Eval failed");
        assert_eq!(result, Value::Number(5050.0)); // Sum of 1..100
    }

    #[test]
    fn test_turing_completeness_collatz() {
        // The Collatz conjecture test - unbounded iteration
        let source = r#"
chant collatz_steps(n) then
    weave steps as 0
    weave num as n

    whilst num > 1 then
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
}
