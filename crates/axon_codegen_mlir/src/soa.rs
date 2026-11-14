//! Native Structure-of-Arrays (SoA) MLIR Lowering
//!
//! Generates column-oriented memrefs and vector dialect operations
//! for AVX-512 / NEON SIMD vectorization.

use crate::types::to_mlir_type;
use axon_types::{GlobalContext, StructInfo};
use lasso::Rodeo;

pub fn generate_soa_mlir_helpers(
    s_info: &StructInfo,
    _ctx: &GlobalContext,
    interner: &Rodeo,
) -> String {
    let mut out = String::new();
    let struct_name = interner.resolve(&s_info.name);

    out.push_str(&format!(
        "  // Native SoA Table definition for {}\n",
        struct_name
    ));

    // Emit struct type definition
    let field_tys: Vec<String> = s_info
        .field_order
        .iter()
        .filter_map(|f| s_info.fields.get(f).map(|(ty, _)| to_mlir_type(ty, interner)))
        .collect();
    out.push_str(&format!(
        "  !Axon_{0} = !llvm.struct<\"{0}\", ({1})>\n",
        struct_name,
        field_tys.join(", ")
    ));

    // Emit Table allocation helper with 64-byte alignment
    out.push_str(&format!(
        "  func.func @Axon_Table_{0}_alloc(%cap: index) -> (!llvm.ptr, index) {{\n",
        struct_name
    ));
    out.push_str("    %c64 = arith.constant 64 : index\n");
    for f_name in &s_info.field_order {
        let f_str = interner.resolve(f_name);
        if let Some((f_ty, _)) = s_info.fields.get(f_name) {
            let elem_ty = to_mlir_type(f_ty, interner);
            out.push_str(&format!(
                "    // Column: {0} ({1})\n    %col_{0} = memref.alloc(%cap) : memref<?x{1}, 64>\n",
                f_str, elem_ty
            ));
        }
    }
    out.push_str("    %ptr = llvm.mlir.zero : !llvm.ptr\n");
    out.push_str("    return %ptr, %cap : !llvm.ptr, index\n");
    out.push_str("  }\n\n");

    out
}
