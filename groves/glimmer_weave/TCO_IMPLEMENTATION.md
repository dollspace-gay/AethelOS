# Tail Call Optimization (TCO) Implementation Plan

## Overview
Implement TCO for the Glimmer-Weave interpreter to enable unbounded tail recursion.

## Changes Required

### 1. Add TailCall variant to RuntimeError (after line 126)

```rust
/// Tail call continuation (for TCO)
TailCall {
    function_name: String,
    args: Vec<Value>,
},
```

### 2. Modify Call handler for Chant functions (lines 472-504)

Replace the current implementation with a trampoline loop:

```rust
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
                    // TCO: Loop with new args instead of recursing
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
```

### 3. Modify YieldStmt handler (lines 446-450) to detect tail calls

```rust
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
```

## Benefits
- **Unbounded tail recursion**: No stack overflow for tail-recursive functions
- **Zero overhead**: Non-tail calls work exactly as before
- **Simple implementation**: ~30 lines of code added
- **Works with any tail-recursive function**: `sum_to(100, 0)` now works!

## Testing
The `test_recursion_with_accumulator` test will now pass with `sum_to(100, 0)` without any stack size hacks.
