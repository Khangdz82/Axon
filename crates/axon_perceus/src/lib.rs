//! axon_perceus: Perceus IR, Liveness Analysis, In-Place Memory Reuse (FBIP),
//! and Refinement-Driven Bounds Check Elision.

pub mod builder;
pub mod ir;
pub mod liveness;
pub mod refinement;
pub mod reuse;
pub mod size;

pub use builder::IrBuilder;
pub use ir::*;
pub use liveness::LivenessPass;
pub use refinement::{RefinementPass, RefinementStats};
pub use reuse::{MemoryReusePass, ReuseStats};

use axon_ast::{FrozenAst, FuncDef, Program};
use axon_types::GlobalContext;
use lasso::Rodeo;
use rayon::prelude::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct PerceusStats {
    pub total_functions: usize,
    pub reused_allocations: usize,
    pub total_drops: usize,
    pub total_allocs: usize,
    pub elided_bounds_checks: usize,
    pub checked_bounds_checks: usize,
}

/// Lower a single function into Perceus IR and apply memory reuse & refinement passes.
/// Fully self-contained for 100% Local Reasoning.
pub fn lower_and_optimize_function(
    func: &FuncDef<'_>,
    ast: FrozenAst<'_, '_>,
    ctx: &GlobalContext,
    interner: &Rodeo,
) -> (IrFunction, PerceusStats) {
    let mut builder = IrBuilder::new(ctx, interner);
    let mut ir_func = builder.build_function(func, ast);

    // Pass 1: Liveness Analysis & Drop Insertion
    let liveness = LivenessPass::new(ctx);
    liveness.run(&mut ir_func);

    // Pass 2: Perceus FBIP In-Place Memory Reuse
    let reuse = MemoryReusePass::new();
    let reuse_stats = reuse.run(&mut ir_func);

    // Pass 3: Refinement Bounds Check Elision
    let refinement = RefinementPass::new(interner);
    let ref_stats = refinement.run(&mut ir_func);

    let stats = PerceusStats {
        total_functions: 1,
        reused_allocations: reuse_stats.reused_allocations,
        total_drops: reuse_stats.total_drops,
        total_allocs: reuse_stats.total_allocs,
        elided_bounds_checks: ref_stats.elided_bounds_checks,
        checked_bounds_checks: ref_stats.checked_bounds_checks,
    };

    (ir_func, stats)
}

/// Compile an entire typed AST program into optimized Perceus IR in parallel using Rayon.
pub fn compile_to_perceus(
    prog: &Program<'_>,
    ast: FrozenAst<'_, '_>,
    ctx: &GlobalContext,
    interner: &Rodeo,
) -> (IrProgram, PerceusStats) {
    let results: Vec<(IrFunction, PerceusStats)> = prog
        .functions
        .par_iter()
        .map(|f| lower_and_optimize_function(f, ast, ctx, interner))
        .collect();

    let mut ir_functions = Vec::with_capacity(results.len());
    let mut agg_stats = PerceusStats {
        total_functions: results.len(),
        ..Default::default()
    };

    for (func, stats) in results {
        ir_functions.push(func);
        agg_stats.reused_allocations += stats.reused_allocations;
        agg_stats.total_drops += stats.total_drops;
        agg_stats.total_allocs += stats.total_allocs;
        agg_stats.elided_bounds_checks += stats.elided_bounds_checks;
        agg_stats.checked_bounds_checks += stats.checked_bounds_checks;
    }

    (
        IrProgram {
            functions: ir_functions,
            ctx: ctx.clone(),
        },
        agg_stats,
    )
}
