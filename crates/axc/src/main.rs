//! Axon Compiler CLI Driver (`axc`)

use axon_ast::AstArena;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: axc <check|ir|build|run> <file.ax>");
        process::exit(1);
    }

    let cmd = &args[1];
    if cmd == "--version" || cmd == "-v" {
        println!("axc 0.1.0 (Axon Systems Language Compiler)");
        return;
    }

    if args.len() < 3 {
        eprintln!("Error: missing input file");
        process::exit(1);
    }

    let file_path = &args[2];
    let src = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    let mut sm = SourceMap::new();
    let file_id = sm.add_file(file_path.clone(), src.clone());

    let mut interner = Rodeo::default();
    let lexer = Lexer::new(&src, file_id, &mut interner);
    let tokens = match lexer.tokenize_all() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", sm.format_error(e.span, &e.msg));
            process::exit(1);
        }
    };

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let prog = {
        let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
        match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}", sm.format_error(e.span, &e.msg));
                process::exit(1);
            }
        }
    };

    let ctx = {
        let mut type_checker = TypeChecker::new(&arena, &interner);
        if let Err(errors) = type_checker.check_program(&prog) {
            for err in errors {
                eprintln!("{}", sm.format_error(err.span, &err.msg));
            }
            process::exit(1);
        }
        type_checker.ctx.clone()
    };

    let frozen = arena.freeze();
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    if cmd == "ir" || cmd == "emit-ir" {
        println!("=== PERCEUS IR: {} ===", file_path);
        for func in &ir_prog.functions {
            print!("{}", func.display(&interner));
        }
        println!("=== PERCEUS STATS ===");
        println!("  Functions:             {}", stats.total_functions);
        println!("  FBIP In-Place Reuses:  {}", stats.reused_allocations);
        println!("  Elided Bounds Checks:  {}", stats.elided_bounds_checks);
        println!("  Checked Bounds Checks: {}", stats.checked_bounds_checks);
        return;
    }

    println!(
        "[+] Successfully checked '{}': {} struct(s), {} function(s) (Local Reasoning verified)",
        file_path,
        prog.structs.len(),
        prog.functions.len()
    );
    println!(
        "    [FBIP] {} in-place allocation reuse(s) | [Refinement] {} bounds check(s) elided, {} checked",
        stats.reused_allocations, stats.elided_bounds_checks, stats.checked_bounds_checks
    );
}
