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
