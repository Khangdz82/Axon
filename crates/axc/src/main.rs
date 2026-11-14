//! Axon Compiler CLI Driver (`axc`)

use axon_ast::AstArena;
use axon_codegen_c::generate_c_program;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: axc <check|ir|emit-c|build|run> <file.ax> [-o <output>]");
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
