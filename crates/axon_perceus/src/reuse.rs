//! Perceus In-Place Memory Reuse (FBIP) Pass
//!
//! Merges pairs of `Drop` and `Alloc` with compatible byte size into
//! `ReuseToken` and `AllocInPlace`, eliminating heap allocations and frees.

use crate::ir::*;
use axon_types::Type;

#[derive(Debug, Default, Clone, Copy)]
pub struct ReuseStats {
    pub reused_allocations: usize,
    pub total_drops: usize,
    pub total_allocs: usize,
}

#[derive(Default)]
pub struct MemoryReusePass;

impl MemoryReusePass {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, func: &mut IrFunction) -> ReuseStats {
        let mut stats = ReuseStats::default();

        for block in &mut func.blocks {
            self.process_block(block, &mut func.var_types, &mut stats);
        }

        stats
    }

    fn process_block(
        &self,
        block: &mut BasicBlock,
        var_types: &mut Vec<Type>,
        stats: &mut ReuseStats,
    ) {
        // Track: (index in block.instructions, dropped_var, byte_size, span)
        let mut available_drops: Vec<(usize, IrVar, usize, axon_span::ByteSpan)> = Vec::new();

        for i in 0..block.instructions.len() {
            match &block.instructions[i] {
                Instruction::Drop {
                    var,
                    byte_size,
                    span,
                } => {
                    stats.total_drops += 1;
                    if *byte_size > 0 {
                        available_drops.push((i, *var, *byte_size, *span));
                    }
                }
                Instruction::Assign {
                    op: IrOp::Alloc { byte_size, .. },
                    ..
                } => {
                    stats.total_allocs += 1;
                    let needed_size = *byte_size;

                    // Find a compatible dropped slot (slot size >= needed size)
                    if let Some(match_idx) = available_drops
                        .iter()
                        .rposition(|(_, _, d_size, _)| *d_size >= needed_size)
                    {
                        let (drop_inst_idx, dropped_var, drop_size, drop_span) =
                            available_drops.swap_remove(match_idx);

                        // Allocate a fresh token variable
                        let token_id = var_types.len() as u32;
                        var_types.push(Type::Unknown);
                        let token_var = IrVar(token_id);

                        // 1. Transform the Drop instruction into a ReuseToken instruction
                        block.instructions[drop_inst_idx] = Instruction::ReuseToken {
                            dest: token_var,
                            source: dropped_var,
                            byte_size: drop_size,
                            span: drop_span,
                        };

                        // 2. Transform the Alloc into AllocInPlace
                        let replacement = match &block.instructions[i] {
                            Instruction::Assign {
                                dest,
                                op:
                                    IrOp::Alloc {
                                        type_name,
                                        byte_size,
                                        fields,
                                    },
                                span,
                            } => Some(Instruction::Assign {
                                dest: *dest,
                                op: IrOp::AllocInPlace {
                                    token: token_var,
                                    type_name: *type_name,
                                    byte_size: *byte_size,
                                    fields: fields.clone(),
                                },
                                span: *span,
                            }),
                            _ => None,
                        };

                        if let Some(new_inst) = replacement {
                            block.instructions[i] = new_inst;
                        }

                        stats.reused_allocations += 1;
                    }
                }
                _ => {}
            }
        }
    }
}
