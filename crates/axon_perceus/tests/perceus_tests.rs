//! Integration tests for Milestone M2: Perceus IR & FBIP In-Place Memory Reuse

use axon_ast::{AstArena, BinOp};
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_perceus::{compile_to_perceus, BoundsCheckMode, Instruction, IrOp};
use axon_span::FileId;
use axon_types::TypeChecker;
use bumpalo::Bump;
use lasso::Rodeo;

#[test]
fn test_fbip_in_place_memory_reuse() {
    let src = r#"
    struct Particle {
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
    }

    fn transform(p: Particle) -> Particle {
        let dead = Particle(p.x, p.y, p.vx, p.vy);
        let val = dead.x;
        let fresh = Particle(val, 0.0, 0.0, 0.0);
        return fresh;
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
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    assert_eq!(stats.total_functions, 1);
    assert!(
        stats.reused_allocations >= 1,
        "Expected at least 1 allocation to be reused in-place (FBIP), got: {}",
        stats.reused_allocations
    );

    let fn_ir = &ir_prog.functions[0];
    let mut found_reuse_token = false;
    let mut found_alloc_in_place = false;

    for block in &fn_ir.blocks {
        for inst in &block.instructions {
            match inst {
                Instruction::ReuseToken { byte_size, .. } => {
                    assert_eq!(*byte_size, 16);
                    found_reuse_token = true;
                }
                Instruction::Assign {
                    op: IrOp::AllocInPlace { byte_size, .. },
                    ..
                } => {
                    assert_eq!(*byte_size, 16);
                    found_alloc_in_place = true;
                }
                _ => {}
            }
        }
    }

    assert!(found_reuse_token, "Must emit ReuseToken instruction");
    assert!(
        found_alloc_in_place,
        "Must emit AllocInPlace instruction for reused memory"
    );
}

#[test]
fn test_refinement_bounds_check_elided_where_clause() {
    let src = r#"
    fn safe_get(arr: Table[i64], idx: usize where idx < arr.len) -> i64 {
        return arr[idx];
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
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    assert_eq!(stats.elided_bounds_checks, 1);
    assert_eq!(stats.checked_bounds_checks, 0);

    let fn_ir = &ir_prog.functions[0];
    let mut elided_found = false;
    for block in &fn_ir.blocks {
        for inst in &block.instructions {
            if let Instruction::Assign {
                op: IrOp::GetIndex { bounds_check, .. },
                ..
            } = inst
            {
                if *bounds_check == BoundsCheckMode::Elided {
                    elided_found = true;
                }
            }
        }
    }
    assert!(
        elided_found,
        "Bounds check must be Elided when refinement proof holds"
    );
}

#[test]
fn test_refinement_bounds_check_unconstrained() {
    let src = r#"
    fn unsafe_get(arr: Table[i64], idx: usize) -> i64 {
        return arr[idx];
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
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    assert_eq!(stats.elided_bounds_checks, 0);
    assert_eq!(stats.checked_bounds_checks, 1);

    let fn_ir = &ir_prog.functions[0];
    let mut checked_found = false;
    for block in &fn_ir.blocks {
        for inst in &block.instructions {
            if let Instruction::Assign {
                op: IrOp::GetIndex { bounds_check, .. },
                ..
            } = inst
            {
                if *bounds_check == BoundsCheckMode::Checked {
                    checked_found = true;
                }
            }
        }
    }
    assert!(
        checked_found,
        "Bounds check must remain Checked when index is unconstrained"
    );
}

#[test]
fn test_for_in_loop_bounds_elision() {
    let src = r#"
    struct Particle {
        x: f32,
    }

    fn step(mut swarm: Table[Particle]) {
        for p in swarm {
            p.x += 1.0;
        }
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
    let (_ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    assert!(
        stats.elided_bounds_checks >= 1,
        "For-in loop iteration must elide bounds checks"
    );
}

#[test]
fn test_full_particle_sim_program_m2() {
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
    let (ir_prog, stats) = compile_to_perceus(&prog, frozen, &ctx, &interner);

    assert_eq!(stats.total_functions, 3);
    assert!(
        stats.elided_bounds_checks >= 2,
        "safe_get and step_physics for-loop should elide bounds checks (got {})",
        stats.elided_bounds_checks
    );

    for func in &ir_prog.functions {
        let ir_text = func.display(&interner);
        assert!(!ir_text.is_empty());
    }
}

#[test]
fn test_checked_wrapping_saturating_arithmetic_ir() {
    let src = r#"
    fn math_ops(a: i32, b: i32) -> i32 {
        let w = a +% b;
        let s = a +! b;
        let c = a + b;
        return c;
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

    let fn_ir = &ir_prog.functions[0];
    let mut found_wrap = false;
    let mut found_sat = false;
    let mut found_checked = false;

    for block in &fn_ir.blocks {
        for inst in &block.instructions {
            if let Instruction::Assign {
                op: IrOp::Binary { op, .. },
                ..
            } = inst
            {
                match op {
                    BinOp::AddWrap => found_wrap = true,
                    BinOp::AddSat => found_sat = true,
                    BinOp::Add => found_checked = true,
                    _ => {}
                }
            }
        }
    }

    assert!(found_wrap, "Must preserve AddWrap (+%) in Perceus IR");
    assert!(found_sat, "Must preserve AddSat (+!) in Perceus IR");
    assert!(
        found_checked,
        "Must preserve checked Add (+) in Perceus IR"
    );
}
