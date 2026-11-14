use axon_ast::AstArena;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_span::SourceMap;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;

fn check_source(src: &str) -> Result<(), Vec<String>> {
    let mut sm = SourceMap::new();
    let file_id = sm.add_file("test.ax".into(), src.into());
    let mut interner = Rodeo::default();
    let tokens = Lexer::new(src, file_id, &mut interner)
        .tokenize_all()
        .map_err(|e| vec![e.msg])?;

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut parser = Parser::new(&tokens, &mut arena, &mut interner);
    let prog = parser.parse_program().map_err(|e| vec![e.msg])?;

    let mut checker = TypeChecker::new(&arena, &interner);
    checker
        .check_program(&prog)
        .map_err(|errs| errs.into_iter().map(|e| e.msg).collect())
}

#[test]
fn test_pass_particle_sim() {
    let src = include_str!("../../../tests/pass/particle_sim.ax");
    let res = check_source(src);
    assert!(res.is_ok(), "Expected particle_sim to pass, got: {:?}", res);
}

#[test]
fn test_fail_01_ref_in_struct() {
    let src = include_str!("../../../tests/fail_compile/01_ref_in_struct.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Second-Class Reference Violation")),
        "Expected second-class ref error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_02_use_after_move_spawn() {
    let src = include_str!("../../../tests/fail_compile/02_use_after_move_spawn.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Linear Isolation Violation")),
        "Expected linear isolation error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_03_use_after_move_call() {
    let src = include_str!("../../../tests/fail_compile/03_use_after_move_call.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Linear Isolation Violation")),
        "Expected linear isolation error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_04_mutate_immutable_let() {
    let src = include_str!("../../../tests/fail_compile/04_mutate_immutable_let.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Cannot mutate immutable 'let'")),
        "Expected immutability error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_05_type_mismatch() {
    let src = include_str!("../../../tests/fail_compile/05_type_mismatch.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Type mismatch")),
        "Expected type mismatch error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_06_return_mismatch() {
    let src = include_str!("../../../tests/fail_compile/06_return_mismatch.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Return type mismatch")),
        "Expected return type mismatch error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_07_mutate_param_let() {
    let src = include_str!("../../../tests/fail_compile/07_mutate_param_let.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Cannot mutate immutable 'let'")),
        "Expected immutability error, got: {:?}",
        errs
    );
}

#[test]
fn test_fail_08_return_reference() {
    let src = include_str!("../../../tests/fail_compile/08_return_reference.ax");
    let errs = check_source(src).expect_err("Expected failure");
    assert!(
        errs.iter().any(|e| e.contains("Second-Class Reference Violation")),
        "Expected second-class ref error, got: {:?}",
        errs
    );
}
