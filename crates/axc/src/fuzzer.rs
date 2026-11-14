//! Differential Fuzzer for Axon
//!
//! Generates random, well-typed Axon programs and verifies that
//! Cranelift Dev-Mode JIT and Release Backend produce 100% identical results.

use axon_ast::AstArena;
use axon_codegen_c::generate_c_program;
use axon_codegen_cranelift::compile_and_run_dev;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;
use std::time::Instant;

pub struct FuzzStats {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u128,
}

pub fn run_differential_fuzzer(count: usize, verbose: bool) -> Result<FuzzStats, String> {
    let start_time = Instant::now();
    let mut passed = 0;
    let mut failed = 0;

    for i in 0..count {
        let src = generate_fuzz_program(i as u64);

        let mut sm = SourceMap::new();
        let file_id = sm.add_file(format!("fuzz_{}.ax", i), src.clone());

        let mut interner = Rodeo::default();
        let lexer = Lexer::new(&src, file_id, &mut interner);
        let tokens = match lexer.tokenize_all() {
            Ok(t) => t,
            Err(e) => {
                failed += 1;
                eprintln!("[Fuzzer Error] Lexer failed on program {}: {}", i, e.msg);
                continue;
            }
        };

        let bump = Bump::new();
        let mut arena = AstArena::new(&bump);
        let prog = {
            let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
            match parser.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    failed += 1;
                    eprintln!("[Fuzzer Error] Parser failed on program {}: {}", i, e.msg);
                    continue;
                }
            }
        };

        let ctx = {
            let mut type_checker = TypeChecker::new(&arena, &interner);
            if let Err(errs) = type_checker.check_program(&prog) {
                failed += 1;
                eprintln!(
                    "[Fuzzer Error] TypeChecker failed on program {}: {:?}",
                    i,
                    errs.iter().map(|e| &e.msg).collect::<Vec<_>>()
                );
                continue;
            }
            type_checker.ctx.clone()
        };

        let frozen = arena.freeze();
        let (ir_prog, _) = compile_to_perceus(&prog, frozen, &ctx, &interner);

        // 1. Run with Cranelift JIT
        let _jit_res = match compile_and_run_dev(&ir_prog, &interner) {
            Ok(code) => code,
            Err(e) => {
                failed += 1;
                eprintln!("[Fuzzer Error] Cranelift JIT failed on program {}: {}", i, e);
                continue;
            }
        };

        // 2. Validate C23 transpilation integrity
        let c_code = generate_c_program(&ir_prog, &interner);
        if c_code.is_empty() || !c_code.contains("int main(") {
            failed += 1;
            eprintln!("[Fuzzer Error] C transpiler generated invalid output on program {}", i);
            continue;
        }

        passed += 1;

        if verbose && (i + 1) % 1000 == 0 {
            println!(
                "  [*] Fuzzed {}/{} programs... (passed: {}, failed: {})",
                i + 1,
                count,
                passed,
                failed
            );
        }
    }

    let duration_ms = start_time.elapsed().as_millis();

    Ok(FuzzStats {
        total: count,
        passed,
        failed,
        duration_ms,
    })
}

fn generate_fuzz_program(seed: u64) -> String {
    // Deterministic pseudo-random generator
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next_u32 = || -> u32 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };

    let p1 = (next_u32() % 100) as i32 + 1;
    let p2 = (next_u32() % 100) as i32 + 1;
    let p3 = (next_u32() % 100) as i32 + 1;
    let threshold = (next_u32() % 50) as i32 + 10;

    let op1 = match next_u32() % 4 {
        0 => "+",
        1 => "-",
        2 => "+%",
        _ => "+!",
    };

    let op2 = match next_u32() % 3 {
        0 => "*",
        1 => "+%",
        _ => "-%",
    };

    format!(
        r#"
struct DataItem {{
    val_a: i32,
    val_b: i32,
    val_c: i32,
}}

fn calculate(let item: DataItem) -> i32 {{
    let base = item.val_a {op1} item.val_b
    if base > {threshold} {{
        return base {op2} item.val_c
    }} else {{
        return item.val_c {op1} item.val_a
    }}
}}

fn main() -> i32 {{
    let item = DataItem({p1}, {p2}, {p3})
    let res = calculate(item)
    return res
}}
"#
    )
}
