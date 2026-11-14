use axon_ast::AstArena;
use axon_codegen_mlir::generate_mlir_program;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;

fn compile_to_mlir(src: &str) -> Result<String, String> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file("test.ax".to_string(), src.to_string());

    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, file_id, &mut interner);
    let tokens = lexer.tokenize_all().map_err(|e| e.msg)?;

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
    let prog = parser.parse_program().map_err(|e| e.msg)?;

    let mut type_checker = TypeChecker::new(&arena, &interner);
    type_checker.check_program(&prog).map_err(|errs| {
        errs.into_iter()
            .map(|e| e.msg)
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let frozen = arena.freeze();
    let (ir_prog, _) = compile_to_perceus(&prog, frozen, &type_checker.ctx, &interner);

    generate_mlir_program(&ir_prog, &interner).map_err(|e| e.to_string())
}

#[test]
fn test_mlir_emission_basic() {
    let code = r#"
        fn add(let a: i32, let b: i32) -> i32 {
            return a + b
        }

        fn main() -> i32 {
            return add(10, 32)
        }
    "#;
    let mlir = compile_to_mlir(code).expect("MLIR generation failed");
    assert!(mlir.contains("module @axon_module"));
    assert!(mlir.contains("func.func @add(%a: i32, %b: i32) -> i32"));
    assert!(mlir.contains("arith.addi"));
    assert!(mlir.contains("func.call @add"));
}

#[test]
fn test_mlir_soa_table_and_fbip() {
    let code = r#"
        struct Particle {
            pos_x: f32,
            pos_y: f32,
            vel_x: f32,
            vel_y: f32,
        }

        fn main() -> i32 {
            let tbl = Table[Particle]::with_capacity(100)
            return 0
        }
    "#;
    let mlir = compile_to_mlir(code).expect("MLIR generation failed");
    assert!(mlir.contains("!Axon_Particle = !llvm.struct<\"Particle\""));
    assert!(mlir.contains("func.func @Axon_Table_Particle_alloc(%cap: index)"));
    assert!(mlir.contains("memref.alloc(%cap) : memref<?xf32, 64>"));
}

#[test]
fn test_mlir_wrapping_and_saturating() {
    let code = r#"
        fn main() -> i32 {
            let x: i32 = 100 +% 200
            let y: i32 = 50 +! 25
            return x - y
        }
    "#;
    let mlir = compile_to_mlir(code).expect("MLIR generation failed");
    assert!(mlir.contains("overflow = \"wrap\""));
    assert!(mlir.contains("overflow = \"saturate\""));
}
