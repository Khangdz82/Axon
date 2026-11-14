use axon_ast::AstArena;
use axon_codegen_cranelift::compile_and_run_dev;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::compile_to_perceus;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;

fn run_axon_dev(src: &str) -> Result<i64, String> {
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

    compile_and_run_dev(&ir_prog, &interner).map_err(|e| e.to_string())
}

#[test]
fn test_jit_arithmetic_basic() {
    let code = r#"
        fn main() -> i32 {
            let a = 40
            let b = 2
            return a + b
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 42);
}

#[test]
fn test_jit_wrapping_and_saturating() {
    let code = r#"
        fn main() -> i32 {
            let x: i32 = 100 +% 200
            let y: i32 = 50 +! 25
            return x - y
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 300 - 75);
}

#[test]
fn test_jit_branching_if_else() {
    let code = r#"
        fn check_val(let n: i32) -> i32 {
            if n > 10 {
                return 100
            } else {
                return 200
            }
        }

        fn main() -> i32 {
            let r1 = check_val(15)
            let r2 = check_val(5)
            return r1 + r2
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 300);
}

#[test]
fn test_jit_struct_fields() {
    let code = r#"
        struct Particle {
            x: i32,
            y: i32,
            mass: i32,
        }

        fn main() -> i32 {
            let p = Particle(10, 20, 5)
            return p.x + p.y + p.mass
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 35);
}

#[test]
fn test_jit_for_loop_table() {
    let code = r#"
        struct Particle {
            x: f32,
            y: f32,
        }

        fn main() -> i32 {
            let tbl = Table[Particle]::with_capacity(4)
            return 0
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 0);
}

#[test]
fn test_jit_function_calls() {
    let code = r#"
        fn add3(let a: i32, let b: i32, let c: i32) -> i32 {
            return a + b + c
        }

        fn mul2(let x: i32) -> i32 {
            return x * 2
        }

        fn main() -> i32 {
            let sum = add3(10, 20, 30)
            return mul2(sum)
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 120);
}

#[test]
fn test_jit_fbip_reuse() {
    let code = r#"
        struct Vector3 {
            x: i32,
            y: i32,
            z: i32,
        }

        fn transform(let v: Vector3) -> Vector3 {
            let updated = Vector3(v.x + 1, v.y + 2, v.z + 3)
            return updated
        }

        fn main() -> i32 {
            let v1 = Vector3(10, 20, 30)
            let v2 = transform(move v1)
            return v2.x + v2.y + v2.z
        }
    "#;
    let res = run_axon_dev(code).expect("JIT execution failed");
    assert_eq!(res, 11 + 22 + 33);
}
