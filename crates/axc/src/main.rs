//! Axon Compiler CLI Driver (`axc`)

mod fuzzer;

use axon_ast::AstArena;
use axon_codegen_c::generate_c_program;
use axon_codegen_cranelift::compile_and_run_dev;
use axon_codegen_mlir::generate_mlir_program;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use fuzzer::run_differential_fuzzer;
use lasso::Rodeo;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: axc <check|ir|emit-c|emit-mlir|build|run|fuzz> <file.ax> [--dev|--release] [-o <output>]");
        process::exit(1);
    }

    let cmd = &args[1];
    if cmd == "--version" || cmd == "-v" {
        println!("axc 0.1.0 (Axon Systems Language Compiler - Dual-Backend Cranelift/MLIR)");
        return;
    }

    if cmd == "fuzz" {
        let count = if args.len() >= 3 {
            if args[2] == "--count" && args.len() >= 4 {
                args[3].parse::<usize>().unwrap_or(10_000)
            } else {
                args[2].parse::<usize>().unwrap_or(10_000)
            }
        } else {
            10_000
        };

        println!("[*] Starting Differential Fuzzer ({} programs)...", count);
        match run_differential_fuzzer(count, true) {
            Ok(stats) => {
                println!("=== DIFFERENTIAL FUZZER RESULTS ===");
                println!("  Total programs:    {}", stats.total);
                println!("  Passed (100% OK):  {}", stats.passed);
                println!("  Failed:            {}", stats.failed);
                println!("  Total time:        {:.2}s ({:.3}ms per program)", stats.duration_ms as f64 / 1000.0, stats.duration_ms as f64 / stats.total as f64);
                if stats.failed == 0 {
                    println!("[+] Differential Fuzzer Gate M5 PASSED: 100% equivalence between backends.");
                    process::exit(0);
                } else {
                    eprintln!("[-] Differential Fuzzer FAILED: {} mismatches found.", stats.failed);
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Fuzzer execution error: {}", e);
                process::exit(1);
            }
        }
    }

    // Parse options: --dev or --release
    let is_dev = args.iter().any(|a| a == "--dev");
    let _is_release = args.iter().any(|a| a == "--release");

    // Filter file_path (first argument after cmd that is not a flag)
    let file_path = args.iter().skip(2).find(|a| !a.starts_with('-'));
    let file_path = match file_path {
        Some(fp) => fp,
        None => {
            eprintln!("Error: missing input file");
            process::exit(1);
        }
    };

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

    if cmd == "emit-mlir" {
        match generate_mlir_program(&ir_prog, &interner) {
            Ok(mlir) => print!("{}", mlir),
            Err(e) => {
                eprintln!("MLIR generation error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    let c_code = generate_c_program(&ir_prog, &interner);

    if cmd == "emit-c" {
        print!("{}", c_code);
        return;
    }

    if cmd == "check" {
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
        return;
    }

    if cmd == "run" && is_dev {
        // Dev Mode: Cranelift JIT compilation and in-memory execution (< 10ms)
        let t0 = Instant::now();
        match compile_and_run_dev(&ir_prog, &interner) {
            Ok(exit_code) => {
                let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
                println!(
                    "[*] [Cranelift Dev JIT] Completed in {:.2}ms (exit code: {})",
                    elapsed_ms, exit_code
                );
                process::exit(exit_code as i32);
            }
            Err(e) => {
                eprintln!("[-] Cranelift Dev JIT error: {}", e);
                process::exit(1);
            }
        }
    }

    if cmd == "build" || cmd == "run" {
        let out_c_path = Path::new(file_path).with_extension("c");
        let out_exe_path = if cfg!(windows) {
            Path::new(file_path).with_extension("exe")
        } else {
            Path::new(file_path).with_extension("")
        };

        if let Err(e) = fs::write(&out_c_path, &c_code) {
            eprintln!("Error writing '{}': {}", out_c_path.display(), e);
            process::exit(1);
        }

        println!("[+] Transpiled Axon to Native SoA C23: '{}'", out_c_path.display());

        // Compile with host C compiler
        let compile_success = compile_c_to_binary(&out_c_path, &out_exe_path);
        if !compile_success {
            eprintln!("Error: failed to compile '{}' with host C compiler", out_c_path.display());
            process::exit(1);
        }

        println!("[+] Compiled binary: '{}'", out_exe_path.display());

        if cmd == "run" {
            println!("[*] Executing '{}':", out_exe_path.display());
            let status = process::Command::new(&out_exe_path)
                .status()
                .expect("Failed to run binary");
            let code = status.code().unwrap_or(1);
            println!("[*] Process exited with status code: {}", code);
            process::exit(code);
        }
    }
}

fn compile_c_to_binary(c_path: &Path, exe_path: &Path) -> bool {
    // 1. Try MSVC cl.exe via vcvars64.bat if on Windows
    let vcvars_candidates = [
        r#"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"#,
        r#"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"#,
    ];

    for vcvars in &vcvars_candidates {
        if Path::new(vcvars).exists() {
            let temp_bat: PathBuf = env::temp_dir().join("axc_build.bat");
            let bat_content = format!(
                "@call \"{}\" >nul 2>&1\r\ncl /nologo /O2 /Fe:\"{}\" \"{}\"\r\n",
                vcvars,
                exe_path.display(),
                c_path.display()
            );
            if fs::write(&temp_bat, &bat_content).is_ok() {
                let status = process::Command::new("cmd")
                    .args(["/C", temp_bat.to_str().unwrap()])
                    .status();
                let _ = fs::remove_file(temp_bat);
                if let Ok(st) = status {
                    if st.success() {
                        return true;
                    }
                }
            }
        }
    }

    // 2. Try clang / gcc on PATH
    for compiler in ["clang", "gcc", "cc"] {
        let status = process::Command::new(compiler)
            .args([
                "-O3",
                "-std=c2x",
                "-o",
                exe_path.to_str().unwrap(),
                c_path.to_str().unwrap(),
            ])
            .status();
        if let Ok(st) = status {
            if st.success() {
                return true;
            }
        }
    }

    false
}
