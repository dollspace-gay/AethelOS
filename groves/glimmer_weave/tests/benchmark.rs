//! Performance benchmarks for Quicksilver VM vs tree-walking interpreter
//!
//! Run with: `cargo test --test benchmark -- --nocapture --ignored`

use glimmer_weave::{Lexer, Parser, Evaluator};
use std::time::Instant;

/// Run a benchmark comparing interpreter vs VM
fn benchmark(name: &str, source: &str, iterations: usize) {
    use glimmer_weave::bytecode_compiler::compile;
    use glimmer_weave::vm::VM;

    println!("\n=== {} ===", name);
    println!("Source: {}", source);
    println!("Iterations: {}", iterations);

    // Parse once (shared by both approaches)
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parse failed");

    // Benchmark tree-walking interpreter (no compilation, just interpret)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut evaluator = Evaluator::new();
        let _ = evaluator.eval(&ast).expect("Interpreter failed");
    }
    let interpreter_time = start.elapsed();

    // Compile bytecode once (outside the timing loop)
    let chunk = compile(&ast).expect("Compilation failed");

    // Benchmark VM (execute pre-compiled bytecode)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut vm = VM::new();
        let _ = vm.execute(chunk.clone()).expect("VM failed");
    }
    let vm_time = start.elapsed();

    // Calculate speedup
    let speedup = interpreter_time.as_secs_f64() / vm_time.as_secs_f64();

    println!("Interpreter: {:?} ({:.2} µs/iter)", interpreter_time, interpreter_time.as_micros() as f64 / iterations as f64);
    println!("VM:          {:?} ({:.2} µs/iter)", vm_time, vm_time.as_micros() as f64 / iterations as f64);
    println!("Speedup:     {:.2}x", speedup);
}

#[test]
#[ignore] // Run explicitly with: cargo test --test benchmark -- --nocapture --ignored
fn bench_simple_arithmetic() {
    benchmark("Simple Arithmetic", "10 + 20 * 2", 10000);
}

#[test]
#[ignore]
fn bench_complex_expression() {
    benchmark(
        "Complex Expression",
        "((10 + 20) * 3 - 5) / 2 + 100",
        10000
    );
}

#[test]
#[ignore]
fn bench_global_variables() {
    benchmark(
        "Global Variables",
        "bind x to 42\nbind y to 8\nx + y * 2",
        10000
    );
}

#[test]
#[ignore]
fn bench_comparisons() {
    benchmark(
        "Comparison Operations",
        "10 < 20 and 30 > 15 or 5 == 5",
        10000
    );
}

#[test]
#[ignore]
fn bench_fibonacci_expression() {
    // Calculate fibonacci(10) using pure expressions
    benchmark(
        "Fibonacci-like Calculation",
        r#"
bind a to 0
bind b to 1
bind c to b + a
bind d to c + b
bind e to d + c
bind f to e + d
bind g to f + e
bind h to g + f
bind i to h + g
bind j to i + h
j
        "#,
        5000
    );
}

#[test]
#[ignore]
fn run_all_benchmarks() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║     Quicksilver VM Performance Benchmarks (OS-114)       ║");
    println!("╚═══════════════════════════════════════════════════════════╝");

    bench_simple_arithmetic();
    bench_complex_expression();
    bench_global_variables();
    bench_comparisons();
    bench_fibonacci_expression();

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                   Benchmark Complete                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
}
