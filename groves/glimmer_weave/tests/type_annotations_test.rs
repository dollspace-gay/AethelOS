//! Tests for Type Annotation Features (OS-112)
//!
//! Tests the Weave-Marks type system: optional static typing with gradual typing support.

use glimmer_weave::lexer::Lexer;
use glimmer_weave::parser::Parser;
use glimmer_weave::eval::{Evaluator, Value};
use glimmer_weave::semantic::analyze;

#[test]
fn test_typed_bind_statement() {
    let source = r#"
        bind x: Number to 42
        bind name: Text to "Alice"
        bind flag: Truth to true
        {x: x, name: name, flag: flag}
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    if let Value::Map(map) = result {
        assert_eq!(map.get("x"), Some(&Value::Number(42.0)));
        assert_eq!(map.get("name"), Some(&Value::Text("Alice".to_string())));
        assert_eq!(map.get("flag"), Some(&Value::Truth(true)));
    } else {
        panic!("Expected Map result");
    }
}

#[test]
fn test_typed_weave_statement() {
    let source = r#"
        weave counter: Number as 0
        set counter to counter + 10
        counter
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(10.0));
}

#[test]
fn test_typed_function_simple() {
    let source = r#"
        chant add(a: Number, b: Number) -> Number then
            yield a + b
        end

        add(10, 32)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_typed_function_with_recursion() {
    let source = r#"
        chant factorial(n: Number) -> Number then
            should n <= 1 then
                yield 1
            otherwise
                yield n * factorial(n - 1)
            end
        end

        factorial(5)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(120.0));
}

#[test]
fn test_mixed_typed_untyped_code() {
    // Gradual typing - mix typed and untyped code
    let source = r#"
        bind x to 10
        bind y: Number to 20
        weave z as 30
        weave w: Number as 40

        chant untyped_func(a, b) then
            yield a + b
        end

        chant typed_func(a: Number, b: Number) -> Number then
            yield a * b
        end

        {
            sum1: untyped_func(x, y),
            sum2: untyped_func(z, w),
            prod1: typed_func(x, y),
            prod2: typed_func(z, w)
        }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    if let Value::Map(map) = result {
        assert_eq!(map.get("sum1"), Some(&Value::Number(30.0)));
        assert_eq!(map.get("sum2"), Some(&Value::Number(70.0)));
        assert_eq!(map.get("prod1"), Some(&Value::Number(200.0)));
        assert_eq!(map.get("prod2"), Some(&Value::Number(1200.0)));
    } else {
        panic!("Expected Map result");
    }
}

#[test]
fn test_semantic_analysis_type_checking() {
    let source = r#"
        bind x: Number to 42
        bind y: Text to "hello"

        chant greet(name: Text) -> Text then
            yield "Hello, " + name
        end

        greet(y)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");

    // Semantic analysis should pass
    let result = analyze(&ast);
    assert!(result.is_ok(), "Expected no semantic errors, got: {:?}", result);

    // Execution should also work
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");
    assert_eq!(result, Value::Text("Hello, hello".to_string()));
}

#[test]
fn test_semantic_analysis_type_error() {
    let source = r#"
        bind x: Number to "not a number"
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");

    // Semantic analysis should detect type error
    let result = analyze(&ast);
    assert!(result.is_err(), "Expected type error to be detected");

    // Check that the error is a type error
    if let Err(errors) = result {
        assert!(
            format!("{:?}", errors).contains("TypeError") ||
            format!("{:?}", errors).contains("expected Number, got Text"),
            "Expected TypeError, got: {:?}", errors
        );
    }
}

#[test]
fn test_typed_list_annotation() {
    let source = r#"
        bind numbers: List<Number> to [1, 2, 3, 4, 5]
        numbers[2]
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(3.0));
}

#[test]
fn test_function_without_return_type() {
    // Return type is optional
    let source = r#"
        chant double(x: Number) then
            yield x * 2
        end

        double(21)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_partial_parameter_typing() {
    // Some parameters typed, others not
    let source = r#"
        chant process(x: Number, y, z: Text) then
            yield x + y
        end

        process(10, 20, "ignored")
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(30.0));
}

#[test]
fn test_typed_fibonacci() {
    let source = r#"
        chant fibonacci(n: Number) -> Number then
            should n <= 1 then
                yield n
            end

            weave a: Number as 0
            weave b: Number as 1
            weave count: Number as 2

            whilst count <= n then
                weave temp: Number as a + b
                set a to b
                set b to temp
                set count to count + 1
            end

            yield b
        end

        fibonacci(10)
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&ast).expect("Eval failed");

    assert_eq!(result, Value::Number(55.0));  // F(10) = 55
}
