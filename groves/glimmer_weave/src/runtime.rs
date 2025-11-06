//! # Glimmer-Weave Runtime Library
//!
//! Standard library functions and utilities for Glimmer-Weave programs.
//!
//! This module provides builtin functions for:
//! - String manipulation (length, slice, concat, upper, lower)
//! - Math operations (abs, sqrt, pow, min, max, floor, ceil, round)
//! - List operations (length, push, pop, reverse, sort)
//! - Map operations (keys, values, has, remove)
//! - Type conversion (to_text, to_number, to_truth)
//! - I/O operations (print, println)

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use crate::eval::{Value, RuntimeError};

/// Math functions abstraction - use std when available (tests), libm when no_std
mod math {
    // Use libm when the feature is enabled
    #[cfg(feature = "use_libm")]
    pub use libm::{sqrt, pow, floor, ceil, round};

    // Use std math functions when std is available (includes tests)
    // This is the default when use_libm feature is not enabled
    #[cfg(not(feature = "use_libm"))]
    pub fn sqrt(x: f64) -> f64 { x.sqrt() }

    #[cfg(not(feature = "use_libm"))]
    pub fn pow(x: f64, y: f64) -> f64 { x.powf(y) }

    #[cfg(not(feature = "use_libm"))]
    pub fn floor(x: f64) -> f64 { x.floor() }

    #[cfg(not(feature = "use_libm"))]
    pub fn ceil(x: f64) -> f64 { x.ceil() }

    #[cfg(not(feature = "use_libm"))]
    pub fn round(x: f64) -> f64 { x.round() }
}

/// Type signature for native function implementations
pub type NativeFn = fn(&[Value]) -> Result<Value, RuntimeError>;

/// Native function wrapper with name and implementation
#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub func: NativeFn,
    pub arity: Option<usize>,  // None = variadic
}

impl NativeFunction {
    pub fn new(name: &str, arity: Option<usize>, func: NativeFn) -> Self {
        NativeFunction {
            name: name.to_string(),
            arity,
            func,
        }
    }
}

impl core::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .finish()
    }
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Get all builtin functions
pub fn get_builtins() -> Vec<NativeFunction> {
    vec![
        // === String Functions ===
        NativeFunction::new("length", Some(1), string_length),
        NativeFunction::new("slice", Some(3), string_slice),
        NativeFunction::new("concat", Some(2), string_concat),
        NativeFunction::new("upper", Some(1), string_upper),
        NativeFunction::new("lower", Some(1), string_lower),
        NativeFunction::new("split", Some(2), string_split),
        NativeFunction::new("join", Some(2), string_join),
        NativeFunction::new("trim", Some(1), string_trim),
        NativeFunction::new("starts_with", Some(2), string_starts_with),
        NativeFunction::new("ends_with", Some(2), string_ends_with),
        NativeFunction::new("contains", Some(2), string_contains),

        // === Math Functions ===
        NativeFunction::new("abs", Some(1), math_abs),
        NativeFunction::new("sqrt", Some(1), math_sqrt),
        NativeFunction::new("pow", Some(2), math_pow),
        NativeFunction::new("min", Some(2), math_min),
        NativeFunction::new("max", Some(2), math_max),
        NativeFunction::new("floor", Some(1), math_floor),
        NativeFunction::new("ceil", Some(1), math_ceil),
        NativeFunction::new("round", Some(1), math_round),

        // === List Functions ===
        NativeFunction::new("list_length", Some(1), list_length),
        NativeFunction::new("list_push", Some(2), list_push),
        NativeFunction::new("list_pop", Some(1), list_pop),
        NativeFunction::new("list_reverse", Some(1), list_reverse),
        NativeFunction::new("list_first", Some(1), list_first),
        NativeFunction::new("list_last", Some(1), list_last),

        // === Map Functions ===
        NativeFunction::new("map_keys", Some(1), map_keys),
        NativeFunction::new("map_values", Some(1), map_values),
        NativeFunction::new("map_has", Some(2), map_has),
        NativeFunction::new("map_size", Some(1), map_size),

        // === Type Conversion ===
        NativeFunction::new("to_text", Some(1), to_text),
        NativeFunction::new("to_number", Some(1), to_number),
        NativeFunction::new("to_truth", Some(1), to_truth),
        NativeFunction::new("type_of", Some(1), type_of),

        // === I/O Functions ===
        NativeFunction::new("print", None, io_print),
        NativeFunction::new("println", None, io_println),
    ]
}

// ============================================================================
// STRING FUNCTIONS
// ============================================================================

fn string_length(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Text(s) => Ok(Value::Number(s.len() as f64)),
        v => Err(RuntimeError::TypeError {
            expected: "Text".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn string_slice(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1], &args[2]) {
        (Value::Text(s), Value::Number(start), Value::Number(end)) => {
            let start = *start as usize;
            let end = *end as usize;

            if start > s.len() || end > s.len() || start > end {
                return Err(RuntimeError::IndexOutOfBounds {
                    index: if start > end { start } else { end },
                    length: s.len(),
                });
            }

            Ok(Value::Text(s[start..end].to_string()))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Number, Number".to_string(),
            got: format!("{}, {}, {}", args[0].type_name(), args[1].type_name(), args[2].type_name()),
        }),
    }
}

fn string_concat(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Text(s1), Value::Text(s2)) => {
            let mut result = s1.clone();
            result.push_str(s2);
            Ok(Value::Text(result))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn string_upper(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Text(s) => {
            let mut result = String::new();
            for c in s.chars() {
                result.push(c.to_ascii_uppercase());
            }
            Ok(Value::Text(result))
        }
        v => Err(RuntimeError::TypeError {
            expected: "Text".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn string_lower(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Text(s) => {
            let mut result = String::new();
            for c in s.chars() {
                result.push(c.to_ascii_lowercase());
            }
            Ok(Value::Text(result))
        }
        v => Err(RuntimeError::TypeError {
            expected: "Text".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn string_split(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Text(s), Value::Text(delimiter)) => {
            let parts: Vec<Value> = s.split(delimiter.as_str())
                .map(|part| Value::Text(part.to_string()))
                .collect();
            Ok(Value::List(parts))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn string_join(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::List(items), Value::Text(separator)) => {
            let strings: Result<Vec<String>, RuntimeError> = items.iter()
                .map(|v| match v {
                    Value::Text(s) => Ok(s.clone()),
                    v => Err(RuntimeError::TypeError {
                        expected: "Text".to_string(),
                        got: v.type_name().to_string(),
                    }),
                })
                .collect();

            let strings = strings?;
            let mut result = String::new();
            for (i, s) in strings.iter().enumerate() {
                if i > 0 {
                    result.push_str(separator);
                }
                result.push_str(s);
            }
            Ok(Value::Text(result))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "List, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn string_trim(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Text(s) => Ok(Value::Text(s.trim().to_string())),
        v => Err(RuntimeError::TypeError {
            expected: "Text".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn string_starts_with(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Text(s), Value::Text(prefix)) => {
            Ok(Value::Truth(s.starts_with(prefix.as_str())))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn string_ends_with(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Text(s), Value::Text(suffix)) => {
            Ok(Value::Truth(s.ends_with(suffix.as_str())))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn string_contains(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Text(s), Value::Text(substring)) => {
            Ok(Value::Truth(s.contains(substring.as_str())))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Text, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

// ============================================================================
// MATH FUNCTIONS
// ============================================================================

fn math_abs(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => Ok(Value::Number(n.abs())),
        v => Err(RuntimeError::TypeError {
            expected: "Number".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn math_sqrt(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => {
            if *n < 0.0 {
                Err(RuntimeError::Custom("Cannot take square root of negative number".to_string()))
            } else {
                Ok(Value::Number(math::sqrt(*n)))
            }
        }
        v => Err(RuntimeError::TypeError {
            expected: "Number".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn math_pow(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Number(base), Value::Number(exp)) => {
            Ok(Value::Number(math::pow(*base, *exp)))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Number, Number".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn math_min(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => {
            Ok(Value::Number(if a < b { *a } else { *b }))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Number, Number".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn math_max(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => {
            Ok(Value::Number(if a > b { *a } else { *b }))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Number, Number".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn math_floor(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => Ok(Value::Number(math::floor(*n))),
        v => Err(RuntimeError::TypeError {
            expected: "Number".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn math_ceil(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => Ok(Value::Number(math::ceil(*n))),
        v => Err(RuntimeError::TypeError {
            expected: "Number".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn math_round(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => Ok(Value::Number(math::round(*n))),
        v => Err(RuntimeError::TypeError {
            expected: "Number".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

// ============================================================================
// LIST FUNCTIONS
// ============================================================================

fn list_length(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => Ok(Value::Number(l.len() as f64)),
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn list_push(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => {
            let mut new_list = l.clone();
            new_list.push(args[1].clone());
            Ok(Value::List(new_list))
        }
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn list_pop(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => {
            if l.is_empty() {
                return Err(RuntimeError::Custom("Cannot pop from empty list".to_string()));
            }
            let mut new_list = l.clone();
            new_list.pop();
            Ok(Value::List(new_list))
        }
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn list_reverse(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => {
            let mut new_list = l.clone();
            new_list.reverse();
            Ok(Value::List(new_list))
        }
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn list_first(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => {
            if l.is_empty() {
                return Err(RuntimeError::Custom("Cannot get first element of empty list".to_string()));
            }
            Ok(l[0].clone())
        }
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn list_last(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::List(l) => {
            if l.is_empty() {
                return Err(RuntimeError::Custom("Cannot get last element of empty list".to_string()));
            }
            Ok(l[l.len() - 1].clone())
        }
        v => Err(RuntimeError::TypeError {
            expected: "List".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

// ============================================================================
// MAP FUNCTIONS
// ============================================================================

fn map_keys(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Map(m) => {
            let keys: Vec<Value> = m.keys()
                .map(|k| Value::Text(k.clone()))
                .collect();
            Ok(Value::List(keys))
        }
        v => Err(RuntimeError::TypeError {
            expected: "Map".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn map_values(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Map(m) => {
            let values: Vec<Value> = m.values()
                .cloned()
                .collect();
            Ok(Value::List(values))
        }
        v => Err(RuntimeError::TypeError {
            expected: "Map".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn map_has(args: &[Value]) -> Result<Value, RuntimeError> {
    match (&args[0], &args[1]) {
        (Value::Map(m), Value::Text(key)) => {
            Ok(Value::Truth(m.contains_key(key)))
        }
        _ => Err(RuntimeError::TypeError {
            expected: "Map, Text".to_string(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

fn map_size(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Map(m) => Ok(Value::Number(m.len() as f64)),
        v => Err(RuntimeError::TypeError {
            expected: "Map".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

// ============================================================================
// TYPE CONVERSION FUNCTIONS
// ============================================================================

fn to_text(args: &[Value]) -> Result<Value, RuntimeError> {
    let text = match &args[0] {
        Value::Number(n) => format!("{}", n),
        Value::Text(s) => s.clone(),
        Value::Truth(b) => if *b { "true".to_string() } else { "false".to_string() },
        Value::Nothing => "nothing".to_string(),
        Value::List(_) => "[List]".to_string(),
        Value::Map(_) => "[Map]".to_string(),
        Value::Chant { .. } => "[Chant]".to_string(),
        Value::NativeChant(native_fn) => format!("[NativeChant:{}]", native_fn.name),
        Value::Capability { .. } => "[Capability]".to_string(),
        Value::Range { .. } => "[Range]".to_string(),
        Value::Outcome { success, value } => {
            // Recursively convert inner value to text
            let inner_text = to_text(&[*value.clone()])?;
            if let Value::Text(inner) = inner_text {
                if *success {
                    format!("Triumph({})", inner)
                } else {
                    format!("Mishap({})", inner)
                }
            } else {
                unreachable!("to_text always returns Text")
            }
        }
        Value::Maybe { present, value } => {
            if *present {
                if let Some(v) = value {
                    let inner_text = to_text(&[*v.clone()])?;
                    if let Value::Text(inner) = inner_text {
                        format!("Present({})", inner)
                    } else {
                        unreachable!("to_text always returns Text")
                    }
                } else {
                    "Present(nothing)".to_string()
                }
            } else {
                "Absent".to_string()
            }
        }
        Value::StructDef { name, .. } => {
            format!("[StructDef:{}]", name)
        }
        Value::StructInstance { struct_name, fields } => {
            // Format as StructName { field1: value1, field2: value2 }
            let mut field_strings = Vec::new();
            for (k, v) in fields.iter() {
                let v_text = to_text(&[v.clone()])?;
                if let Value::Text(s) = v_text {
                    field_strings.push(format!("{}: {}", k, s));
                } else {
                    unreachable!("to_text always returns Text")
                }
            }
            format!("{} {{ {} }}", struct_name, field_strings.join(", "))
        }
    };
    Ok(Value::Text(text))
}

fn to_number(args: &[Value]) -> Result<Value, RuntimeError> {
    match &args[0] {
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Text(s) => {
            s.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| RuntimeError::Custom(format!("Cannot convert '{}' to number", s)))
        }
        Value::Truth(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        v => Err(RuntimeError::TypeError {
            expected: "Number, Text, or Truth".to_string(),
            got: v.type_name().to_string(),
        }),
    }
}

fn to_truth(args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Truth(args[0].is_truthy()))
}

fn type_of(args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Text(args[0].type_name().to_string()))
}

// ============================================================================
// I/O FUNCTIONS
// ============================================================================
//
// NOTE: These are stub implementations. Real I/O should be provided by
// the host environment (kernel) via capability-based syscalls.
// For now, these functions are not implemented and will return errors.

fn io_print(_args: &[Value]) -> Result<Value, RuntimeError> {
    Err(RuntimeError::Custom(
        "print() requires kernel I/O capabilities - call from kernel context only".to_string()
    ))
}

fn io_println(_args: &[Value]) -> Result<Value, RuntimeError> {
    Err(RuntimeError::Custom(
        "println() requires kernel I/O capabilities - call from kernel context only".to_string()
    ))
}
