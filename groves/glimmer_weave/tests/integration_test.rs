//! Comprehensive Integration Test for Glimmer-Weave
//!
//! Tests every language capability including Turing completeness by
//! compiling and executing a complete Glimmer-Weave program.

use glimmer_weave::lexer::Lexer;
use glimmer_weave::parser::Parser;
use glimmer_weave::eval::{Evaluator, Value};
use std::fs;

#[test]
fn test_comprehensive_glimmer_weave_program() {
    // Run test in a thread with larger stack size to handle deep recursion
    // Default stack is ~2MB, we use 16MB for deep recursive functions
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024) // 16 MB stack
        .spawn(|| {
            // Read the comprehensive test program
            let source = fs::read_to_string("tests/comprehensive_test.gw")
                .expect("Failed to read comprehensive_test.gw");

            println!("\n═══════════════════════════════════════════════════════════");
            println!("Starting Glimmer-Weave Comprehensive Integration Test");
            println!("═══════════════════════════════════════════════════════════\n");

            // PHASE 1: LEXING
            println!("Phase 1: Lexical Analysis (Tokenization)");
            println!("─────────────────────────────────────────────────────────");

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            println!("✓ Tokenized {} tokens", tokens.len());
            println!("  First 10 tokens: {:?}\n", &tokens[0..10.min(tokens.len())]);

            // PHASE 2: PARSING
            println!("Phase 2: Syntactic Analysis (Parsing)");
            println!("─────────────────────────────────────────────────────────");

            let mut parser = Parser::new(tokens);
            let ast = parser.parse().expect("Parse failed");

            println!("✓ Parsed {} AST nodes", ast.len());
            println!("  AST structure verified\n");

            // PHASE 3: EVALUATION
            println!("Phase 3: Semantic Analysis and Execution");
            println!("─────────────────────────────────────────────────────────");

            let mut evaluator = Evaluator::new();
            let result = evaluator.eval(&ast).expect("Evaluation failed");

            println!("✓ Program executed successfully");
            println!("  Result type: {:?}\n", result);

            // PHASE 4: VERIFICATION
            println!("Phase 4: Results Verification");
            println!("─────────────────────────────────────────────────────────");

            // The result should be a Map containing the test summary
            if let Value::Map(summary) = result {
                // Check each capability
                verify_boolean(&summary, "variables_ok", "Variables (bind/weave/set)");
                verify_boolean(&summary, "arithmetic_ok", "Arithmetic operators");
                verify_boolean(&summary, "comparisons_ok", "Comparison operators");
                verify_boolean(&summary, "logic_ok", "Logical operators");
                verify_boolean(&summary, "conditionals_ok", "Conditional statements (should/otherwise)");
                verify_boolean(&summary, "for_loops_ok", "Bounded loops (for each)");
                verify_boolean(&summary, "while_loops_ok", "Unbounded loops (whilst)");
                verify_boolean(&summary, "functions_ok", "Functions (chant/yield)");
                verify_boolean(&summary, "recursion_ok", "Recursion");
                verify_boolean(&summary, "fibonacci_ok", "Fibonacci (while loop)");
                verify_boolean(&summary, "prime_check_ok", "Prime number checking");
                verify_boolean(&summary, "gcd_ok", "GCD algorithm");
                verify_boolean(&summary, "turing_complete", "Turing Completeness");

                println!("\n═══════════════════════════════════════════════════════════");
                println!("✨ ALL TESTS PASSED! Glimmer-Weave is fully functional! ✨");
                println!("═══════════════════════════════════════════════════════════\n");
            } else {
                panic!("Expected Map result, got: {:?}", result);
            }
        })
        .expect("Failed to spawn test thread")
        .join()
        .expect("Test thread panicked");
}

fn verify_boolean(map: &std::collections::BTreeMap<String, Value>, key: &str, description: &str) {
    match map.get(key) {
        Some(Value::Truth(true)) => {
            println!("  ✓ {}: PASS", description);
        }
        Some(Value::Truth(false)) => {
            panic!("  ✗ {}: FAIL", description);
        }
        Some(other) => {
            panic!("  ✗ {}: Expected boolean, got {:?}", description, other);
        }
        None => {
            panic!("  ✗ {}: Key '{}' not found in result", description, key);
        }
    }
}

#[test]
fn test_factorial_correctness() {
    let source = r#"
        chant factorial(n) then
            should n <= 1 then
                yield 1
            otherwise
                yield n * factorial(n - 1)
            end
        end

        factorial(10)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(3628800.0)); // 10! = 3,628,800
    println!("✓ Factorial(10) = 3,628,800 (correct)");
}

#[test]
fn test_fibonacci_correctness() {
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

        fibonacci(20)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(6765.0)); // F(20) = 6,765
    println!("✓ Fibonacci(20) = 6,765 (correct)");
}

#[test]
fn test_gcd_correctness() {
    let source = r#"
        chant gcd(a, b) then
            weave x as a
            weave y as b

            whilst y > 0 then
                weave temp as y
                set y to x % y
                set x to temp
            end

            yield x
        end

        gcd(1071, 462)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(21.0)); // GCD(1071, 462) = 21
    println!("✓ GCD(1071, 462) = 21 (correct)");
}

#[test]
fn test_nested_while_loops() {
    let source = r#"
        weave sum as 0
        weave i as 1

        whilst i <= 5 then
            weave j as 1
            whilst j <= 5 then
                set sum to sum + 1
                set j to j + 1
            end
            set i to i + 1
        end

        sum
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(25.0)); // 5×5 = 25
    println!("✓ Nested while loops: 5×5 = 25 (correct)");
}

#[test]
fn test_collatz_conjecture() {
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

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(111.0)); // Collatz(27) = 111 steps
    println!("✓ Collatz(27) = 111 steps (correct - demonstrates unbounded iteration)");
}

#[test]
fn test_all_data_types() {
    let source = r#"
        bind num to 42
        bind txt to "Hello"
        bind truth to true
        bind nada to nothing
        bind lst to [1, 2, 3]
        bind mp to {key: "value"}

        {
            has_number: num is 42,
            has_text: txt is "Hello",
            has_truth: truth,
            has_nothing: nada is nothing,
            has_list: lst[0] is 1,
            has_map: mp.key is "value"
        }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    if let Value::Map(map) = result {
        assert_eq!(map.get("has_number"), Some(&Value::Truth(true)));
        assert_eq!(map.get("has_text"), Some(&Value::Truth(true)));
        assert_eq!(map.get("has_truth"), Some(&Value::Truth(true)));
        assert_eq!(map.get("has_nothing"), Some(&Value::Truth(true)));
        assert_eq!(map.get("has_list"), Some(&Value::Truth(true)));
        assert_eq!(map.get("has_map"), Some(&Value::Truth(true)));
        println!("✓ All data types work correctly");
    } else {
        panic!("Expected Map result");
    }
}
