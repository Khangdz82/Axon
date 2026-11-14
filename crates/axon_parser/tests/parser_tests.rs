use axon_ast::AstArena;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_span::SourceMap;
use bumpalo::Bump;
use lasso::Rodeo;

fn parse_valid(src: &str) {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file("test.ax".into(), src.into());
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, file_id, &mut interner);
    let tokens = lexer.tokenize_all().expect("Lexing failed");

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
    let res = parser.parse_program();
    assert!(res.is_ok(), "Expected valid parse, got: {:?}", res.err());
}

fn parse_error_at(src: &str, expected_line: usize, expected_col: usize) {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file("test.ax".into(), src.into());
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, file_id, &mut interner);
    let tokens = match lexer.tokenize_all() {
        Ok(t) => t,
        Err(e) => {
            let file = sm.get(file_id).unwrap();
            let (l, c) = file.line_col(e.span.start);
            assert_eq!((l, c), (expected_line, expected_col), "Lex error location mismatch");
            return;
        }
    };

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
    let err = parser.parse_program().expect_err("Expected parse error");

    let file = sm.get(file_id).unwrap();
    let (line, col) = file.line_col(err.span.start);
    assert_eq!(
        (line, col),
        (expected_line, expected_col),
        "Error location mismatch. Msg: {}",
        err.msg
    );
}

// =========================================================================
// 30+ VALID SYNTAX TESTS
// =========================================================================

#[test]
fn test_valid_01_empty_struct() {
    parse_valid("struct Empty {}");
}

#[test]
fn test_valid_02_struct_with_primitives() {
    parse_valid("struct Particle { x: f32, y: f32, id: u64 }");
}

#[test]
fn test_valid_03_struct_with_table() {
    parse_valid("struct Mesh { vertices: Table[Vec3], indices: Table[u32] }");
}

#[test]
fn test_valid_04_fn_void_empty() {
    parse_valid("fn noop() {}");
}

#[test]
fn test_valid_05_fn_params_all_intents() {
    parse_valid("fn process(let a: i32, mut b: Table[Particle], move c: Table[Particle], isolated d: Table[Particle]) {}");
}

#[test]
fn test_valid_06_fn_return_primitive() {
    parse_valid("fn add(a: i32, b: i32) -> i32 { return a + b; }");
}

#[test]
fn test_valid_07_fn_return_table() {
    parse_valid("fn make_world() -> Table[Particle] { return Table[Particle]::with_capacity(100); }");
}

#[test]
fn test_valid_08_refinement_type_param() {
    parse_valid("fn get(arr: Table[i64], idx: usize where idx < arr.len) -> i64 { return arr[idx]; }");
}

#[test]
fn test_valid_09_refinement_type_complex() {
    parse_valid("fn clamp(val: i32 where val >= 0 && val <= 100) -> i32 { return val; }");
}

#[test]
fn test_valid_10_let_typed() {
    parse_valid("fn test() { let x: f32 = 3.14; }");
}

#[test]
fn test_valid_11_let_inferred() {
    parse_valid("fn test() { let x = 42; }");
}

#[test]
fn test_valid_12_isolated_let() {
    parse_valid("fn test() { isolated world = Table[Particle]::with_capacity(1000); }");
}

#[test]
fn test_valid_13_basic_arithmetic() {
    parse_valid("fn math() { let a = 1 + 2 * 3 - 4 / 2 % 1; }");
}

#[test]
fn test_valid_14_wrapping_arithmetic() {
    parse_valid("fn math() { let a = 100 +% 200 -% 50 *% 2; }");
}

#[test]
fn test_valid_15_saturating_arithmetic() {
    parse_valid("fn math() { let a = 100 +! 200 -! 50 *! 2; }");
}

#[test]
fn test_valid_16_logical_ops() {
    parse_valid("fn logic() { let ok = true && false || true; }");
}

#[test]
fn test_valid_17_comparisons() {
    parse_valid("fn comp() { let b = (1 < 2) && (3 <= 4) && (5 > 4) && (6 >= 6) && (7 == 7) && (8 != 9); }");
}

#[test]
fn test_valid_18_field_access() {
    parse_valid("fn fields() { p.x += p.vx * dt; }");
}

#[test]
fn test_valid_19_index_access() {
    parse_valid("fn indexing() { let item = arr[0]; arr[1] = 42; }");
}

#[test]
fn test_valid_20_method_call() {
    parse_valid("fn methods() { let res = worker.join(); }");
}

#[test]
fn test_valid_21_static_method_call() {
    parse_valid("fn static_call() { let t = Table[Particle]::with_capacity(1_000_000); }");
}

#[test]
fn test_valid_22_call_with_intents() {
    parse_valid("fn calls() { step_physics(mut swarm, 0.016); pass(move world); }");
}

#[test]
fn test_valid_23_return_move() {
    parse_valid("fn ret_move() -> Table[Particle] { return move world; }");
}

#[test]
fn test_valid_24_for_in_loop() {
    parse_valid("fn loops() { for p in swarm { p.x += p.vx * dt; } }");
}

#[test]
fn test_valid_25_while_loop() {
    parse_valid("fn loops() { while i < 100 { i += 1; } }");
}

#[test]
fn test_valid_26_if_then() {
    parse_valid("fn branch() { if x > 0 { return 1; } }");
}

#[test]
fn test_valid_27_if_else() {
    parse_valid("fn branch() { if x > 0 { return 1; } else { return -1; } }");
}

#[test]
fn test_valid_28_if_else_if() {
    parse_valid("fn branch() { if x > 0 { return 1; } else if x == 0 { return 0; } else { return -1; } }");
}

#[test]
fn test_valid_29_spawn_block() {
    parse_valid("fn conc() { let job = spawn { step_physics(mut world, 0.016); return move world; }; }");
}

#[test]
fn test_valid_30_full_particle_sim_program() {
    let code = r#"
    struct Particle {
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
    }

    fn step_physics(mut swarm: Table[Particle], dt: f32) {
        for p in swarm {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
    }

    fn safe_get(arr: Table[i64], idx: usize where idx < arr.len) -> i64 {
        return arr[idx];
    }

    fn main() -> i32 {
        isolated world = Table[Particle]::with_capacity(1_000_000);
        let job = spawn {
            step_physics(mut world, 0.016);
            return move world;
        };
        let updated_world = job.join();
        return 0;
    }
    "#;
    parse_valid(code);
}

#[test]
fn test_valid_31_multiple_structs_and_fns() {
    parse_valid("struct A { x: i32 } struct B { a: A } fn f1() {} fn f2() {}");
}

// =========================================================================
// 15+ SYNTAX ERROR TESTS WITH EXACT LINE/COL REPORTING
// =========================================================================

#[test]
fn test_err_01_missing_struct_name() {
    // line 1, col 8: 'struct {' -> expected ident
    parse_error_at("struct { x: f32 }", 1, 8);
}

#[test]
fn test_err_02_missing_colon_in_field() {
    // line 1, col 13: 'struct P { x f32 }' -> expected colon after ident 'x'
    parse_error_at("struct P { x f32 }", 1, 14);
}

#[test]
fn test_err_03_missing_type_in_field() {
    // line 1, col 16: 'struct P { x: }' -> expected type after colon
    parse_error_at("struct P { x: }", 1, 15);
}

#[test]
fn test_err_04_unclosed_struct_brace() {
    // line 1, col 19: 'struct P { x: f32' (hits EOF at col 18)
    parse_error_at("struct P { x: f32", 1, 18);
}

#[test]
fn test_err_05_missing_fn_name() {
    // line 1, col 4: 'fn () {}'
    parse_error_at("fn () {}", 1, 4);
}

#[test]
fn test_err_06_missing_param_type() {
    // line 1, col 9: 'fn foo(x) {}' -> expected colon
    parse_error_at("fn foo(x) {}", 1, 9);
}

#[test]
fn test_err_07_missing_ret_type() {
    // line 1, col 13: 'fn foo() -> {}' -> expected type name
    parse_error_at("fn foo() -> {}", 1, 13);
}

#[test]
fn test_err_08_unclosed_fn_parens() {
    // line 1, col 15: 'fn foo(x: i32 {}' -> expected RParen
    parse_error_at("fn foo(x: i32 {}", 1, 15);
}

#[test]
fn test_err_09_unclosed_fn_body() {
    // line 2, col 1: hits EOF without closing brace
    parse_error_at("fn foo() {\n    let x = 1;", 2, 15);
}

#[test]
fn test_err_10_missing_in_for_loop() {
    // line 1, col 18: 'fn f() { for p swarm {} }' -> expected in
    parse_error_at("fn f() { for p swarm {} }", 1, 16);
}

#[test]
fn test_err_11_unclosed_table_bracket() {
    // line 1, col 17: 'struct S { t: Table[Particle }'
    parse_error_at("struct S { t: Table[Particle }", 1, 30);
}

#[test]
fn test_err_12_incomplete_binary_op() {
    // line 1, col 23: 'fn f() { let x = 1 + ; }'
    parse_error_at("fn f() { let x = 1 + ; }", 1, 22);
}

#[test]
fn test_err_13_incomplete_assignment() {
    // line 1, col 16: 'fn f() { x = ; }'
    parse_error_at("fn f() { x = ; }", 1, 14);
}

#[test]
fn test_err_14_invalid_toplevel_token() {
    // line 1, col 1: 'let x = 10;' at top level
    parse_error_at("let x = 10;", 1, 1);
}

#[test]
fn test_err_15_unclosed_spawn_brace() {
    // line 1, col 32: hits EOF
    parse_error_at("fn f() { let j = spawn { return 1; }", 1, 37);
}
