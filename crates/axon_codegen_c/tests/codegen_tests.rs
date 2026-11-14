//! Tests for Milestone M3: Native SoA & C23 Transpiler Backend

use axon_ast::AstArena;
use axon_codegen_c::generate_c_program;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::FileId;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;

#[test]
fn test_soa_table_generation() {
    let src = r#"
    struct Particle {
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
    }

    fn init_table() -> Table[Particle] {
        let t = Table[Particle]::with_capacity(100);
        return t;
    }
    "#;

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut interner = Rodeo::default();
    let tokens = Lexer::new(src, FileId(0), &mut interner)
        .tokenize_all()
        .unwrap();
    let prog = {
        let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
        parser.parse_program().unwrap()
    };

    let ctx = {
        let mut checker = TypeChecker::new(&arena, &interner);
        checker.check_program(&prog).unwrap();
        checker.ctx.clone()
    };

    let frozen = arena.freeze();
    let (ir_prog, _stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    let c_code = generate_c_program(&ir_prog, &interner);

    assert!(
        c_code.contains("typedef struct Axon_Particle"),
        "Must generate AoS Particle struct"
    );
    assert!(
        c_code.contains("typedef struct Axon_Table_Particle"),
        "Must generate Native SoA Table_Particle struct"
    );
    assert!(
        c_code.contains("float* AXON_RESTRICT x;"),
        "Table fields must be separate restricted arrays for SIMD"
    );
    assert!(
        c_code.contains("float* AXON_RESTRICT vx;"),
        "Table fields must be separate restricted arrays for SIMD"
    );
    assert!(
        c_code.contains("axon_aligned_alloc(64"),
        "Table allocations must be 64-byte cache line aligned"
    );
}

#[test]
fn test_particle_sim_c_generation_and_compilation() {
    let src = include_str!("../../../tests/pass/particle_sim.ax");

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut interner = Rodeo::default();
    let tokens = Lexer::new(src, FileId(0), &mut interner)
        .tokenize_all()
        .unwrap();
    let prog = {
        let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
        parser.parse_program().unwrap()
    };

    let ctx = {
        let mut checker = TypeChecker::new(&arena, &interner);
        checker.check_program(&prog).unwrap();
        checker.ctx.clone()
    };

    let frozen = arena.freeze();
    let (ir_prog, _stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    let c_code = generate_c_program(&ir_prog, &interner);

    // Write generated C code to a temp file
    let temp_dir = std::env::temp_dir();
    let c_file_path = temp_dir.join("particle_sim_test.c");
    let exe_file_path = temp_dir.join("particle_sim_test.exe");
    std::fs::write(&c_file_path, &c_code).expect("Failed to write C file");

    // Test compiling with MSVC cl.exe via vcvars64.bat if available
    let vcvars_path = r#"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"#;
    if std::path::Path::new(vcvars_path).exists() {
        let bat_path = temp_dir.join("build_test.bat");
        let bat_content = format!(
            "@call \"{}\" >nul 2>&1\r\ncl /nologo /O2 /Fe:\"{}\" \"{}\"\r\n",
            vcvars_path,
            exe_file_path.display(),
            c_file_path.display()
        );
        std::fs::write(&bat_path, &bat_content).expect("Failed to write bat file");

        let status = std::process::Command::new("cmd")
            .args(["/C", bat_path.to_str().unwrap()])
            .status()
            .expect("Failed to execute build bat");

        assert!(
            status.success(),
            "Generated C code must compile with cl.exe"
        );

        // Run the compiled executable
        let run_status = std::process::Command::new(&exe_file_path)
            .status()
            .expect("Failed to execute compiled binary");

        assert!(
            run_status.success(),
            "Compiled binary must run and exit with code 0"
        );

        // Cleanup
        let _ = std::fs::remove_file(c_file_path);
        let _ = std::fs::remove_file(exe_file_path);
        let _ = std::fs::remove_file(bat_path);
        let _ = std::fs::remove_file(temp_dir.join("particle_sim_test.obj"));
    }
}

#[test]
fn test_multi_fiber_concurrency_compilation_and_execution() {
    let src = include_str!("../../../tests/pass/multi_fiber_concurrency.ax");

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut interner = Rodeo::default();
    let tokens = Lexer::new(src, FileId(0), &mut interner)
        .tokenize_all()
        .unwrap();
    let prog = {
        let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
        parser.parse_program().unwrap()
    };

    let ctx = {
        let mut checker = TypeChecker::new(&arena, &interner);
        checker.check_program(&prog).unwrap();
        checker.ctx.clone()
    };

    let frozen = arena.freeze();
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    // 3 AST functions + 3 outlined fiber functions = 6 functions in IR
    assert_eq!(stats.total_functions, 6);

    let c_code = generate_c_program(&ir_prog, &interner);
    assert!(c_code.contains("axon_scheduler_init(0)"));
    assert!(c_code.contains("axon_scheduler_shutdown()"));
    assert!(c_code.contains("axon_spawn(&Axon_thunk___axon_fiber_0"));
    assert!(c_code.contains("axon_spawn(&Axon_thunk___axon_fiber_1"));
    assert!(c_code.contains("axon_spawn(&Axon_thunk___axon_fiber_2"));

    // Write generated C code to a temp file
    let temp_dir = std::env::temp_dir();
    let c_file_path = temp_dir.join("multi_fiber_test.c");
    let exe_file_path = temp_dir.join("multi_fiber_test.exe");
    std::fs::write(&c_file_path, &c_code).expect("Failed to write C file");

    let vcvars_path = r#"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"#;
    if std::path::Path::new(vcvars_path).exists() {
        let bat_path = temp_dir.join("build_multi_fiber_test.bat");
        let bat_content = format!(
            "@call \"{}\" >nul 2>&1\r\ncl /nologo /O2 /Fe:\"{}\" \"{}\"\r\n",
            vcvars_path,
            exe_file_path.display(),
            c_file_path.display()
        );
        std::fs::write(&bat_path, &bat_content).expect("Failed to write bat file");

        let status = std::process::Command::new("cmd")
            .args(["/C", bat_path.to_str().unwrap()])
            .status()
            .expect("Failed to execute build bat");

        assert!(status.success(), "Generated C code must compile with cl.exe");

        // Run the compiled executable
        let run_status = std::process::Command::new(&exe_file_path)
            .status()
            .expect("Failed to execute compiled binary");

        assert!(run_status.success(), "Multi-fiber binary must run and exit with code 0");

        // Cleanup
        let _ = std::fs::remove_file(c_file_path);
        let _ = std::fs::remove_file(exe_file_path);
        let _ = std::fs::remove_file(bat_path);
        let _ = std::fs::remove_file(temp_dir.join("multi_fiber_test.obj"));
    }
}

#[test]
fn test_work_stealing_and_channel_c_runtime() {
    let test_c_source = r#"
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
"#
    .to_string()
        + axon_codegen_c::runtime_header::RUNTIME_HEADER
        + r#"
void* task_square(void* arg) {
    int64_t x = (int64_t)(intptr_t)arg;
    return (void*)(intptr_t)(x * x);
}

int main(void) {
    axon_scheduler_init(4);

    // Test concurrent task execution & stealing
    Axon_Job* j1 = axon_spawn(task_square, (void*)10);
    Axon_Job* j2 = axon_spawn(task_square, (void*)20);
    Axon_Job* j3 = axon_spawn(task_square, (void*)30);

    int64_t r1 = (int64_t)(intptr_t)axon_join(j1);
    int64_t r2 = (int64_t)(intptr_t)axon_join(j2);
    int64_t r3 = (int64_t)(intptr_t)axon_join(j3);

    assert(r1 == 100);
    assert(r2 == 400);
    assert(r3 == 900);

    // Test lock-free channel for isolated pointer passing
    Axon_Channel* ch = axon_channel_create();
    assert(ch != NULL);
    bool sent = axon_channel_send(ch, (void*)(intptr_t)12345);
    assert(sent);
    void* recv_val = axon_channel_recv(ch);
    assert((int64_t)(intptr_t)recv_val == 12345);
    axon_channel_free(ch);

    axon_scheduler_shutdown();
    return 0;
}
"#;

    let temp_dir = std::env::temp_dir();
    let c_file_path = temp_dir.join("runtime_test.c");
    let exe_file_path = temp_dir.join("runtime_test.exe");
    std::fs::write(&c_file_path, &test_c_source).expect("Failed to write runtime test C file");

    let vcvars_path = r#"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"#;
    if std::path::Path::new(vcvars_path).exists() {
        let bat_path = temp_dir.join("build_runtime_test.bat");
        let bat_content = format!(
            "@call \"{}\" >nul 2>&1\r\ncl /nologo /O2 /Fe:\"{}\" \"{}\"\r\n",
            vcvars_path,
            exe_file_path.display(),
            c_file_path.display()
        );
        std::fs::write(&bat_path, &bat_content).expect("Failed to write bat file");

        let status = std::process::Command::new("cmd")
            .args(["/C", bat_path.to_str().unwrap()])
            .status()
            .expect("Failed to execute build bat");

        assert!(status.success(), "Runtime test C code must compile with cl.exe");

        let run_status = std::process::Command::new(&exe_file_path)
            .status()
            .expect("Failed to execute runtime binary");

        assert!(run_status.success(), "Runtime binary must run and exit with code 0");

        let _ = std::fs::remove_file(c_file_path);
        let _ = std::fs::remove_file(exe_file_path);
        let _ = std::fs::remove_file(bat_path);
        let _ = std::fs::remove_file(temp_dir.join("runtime_test.obj"));
    }
}
