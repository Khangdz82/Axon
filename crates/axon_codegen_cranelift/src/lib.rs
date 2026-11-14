//! Axon Cranelift JIT Dev-Mode Backend
//!
//! Compiles Axon Perceus IR directly to machine code in memory
//! with sub-10ms latency for rapid dev-loop iterations.

pub mod jit;
pub mod types;

use axon_perceus::ir::IrProgram;
use lasso::Rodeo;
pub use jit::CraneliftJit;

#[derive(Debug, thiserror::Error)]
pub enum CraneliftError {
    #[error("Cranelift JIT error: {0}")]
    Jit(String),
}

/// Compile and execute an Axon program in memory using Cranelift JIT.
///
/// Returns the exit code from `main()` (0 if main returns void).
pub fn compile_and_run_dev(
    ir_prog: &IrProgram,
    interner: &Rodeo,
) -> Result<i64, CraneliftError> {
    let jit = CraneliftJit::new().map_err(CraneliftError::Jit)?;
    jit.compile_and_run(ir_prog, interner).map_err(CraneliftError::Jit)
}
