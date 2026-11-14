//! Native Structure-of-Arrays (SoA) code generator
//!
//! Generates cache-friendly column-oriented `Table[T]` layouts
//! where each field is stored in an independent 64-byte aligned array.

use crate::types::to_c_type;
use axon_types::{GlobalContext, StructInfo};
use lasso::Rodeo;

pub fn generate_struct_and_soa(
    s_info: &StructInfo,
    _ctx: &GlobalContext,
    interner: &Rodeo,
) -> String {
    let struct_name = interner.resolve(&s_info.name);
    let mut out = String::new();

    // 1. Regular AoS struct definition
    out.push_str(&format!("typedef struct Axon_{} {{\n", struct_name));
    for f_name in &s_info.field_order {
        if let Some((f_ty, _)) = s_info.fields.get(f_name) {
            let c_ty = to_c_type(f_ty, interner);
            let field_str = interner.resolve(f_name);
            out.push_str(&format!("    {} {};\n", c_ty, field_str));
        }
    }
    out.push_str(&format!("}} Axon_{};\n\n", struct_name));

    // 2. Native SoA Table definition (64-byte aligned column arrays)
    out.push_str(&format!("typedef struct Axon_Table_{} {{\n", struct_name));
    out.push_str("    size_t len;\n");
    out.push_str("    size_t capacity;\n");
    for f_name in &s_info.field_order {
        if let Some((f_ty, _)) = s_info.fields.get(f_name) {
            let c_ty = to_c_type(f_ty, interner);
            let field_str = interner.resolve(f_name);
            out.push_str(&format!(
                "    {}* AXON_RESTRICT {};\n",
                c_ty, field_str
            ));
        }
    }
    out.push_str(&format!("}} Axon_Table_{};\n\n", struct_name));

    // 3. Table factory: with_capacity
    out.push_str(&format!(
        "static inline Axon_Table_{0} Axon_Table_{0}_with_capacity(size_t cap) {{\n",
        struct_name
    ));
    out.push_str(&format!("    Axon_Table_{} tbl;\n", struct_name));
    out.push_str("    tbl.len = 0;\n");
    out.push_str("    tbl.capacity = cap;\n");
    for f_name in &s_info.field_order {
        if let Some((f_ty, _)) = s_info.fields.get(f_name) {
            let c_ty = to_c_type(f_ty, interner);
            let field_str = interner.resolve(f_name);
            out.push_str(&format!(
                "    tbl.{0} = ({1}*)axon_aligned_alloc(64, cap * sizeof({1}));\n",
                field_str, c_ty
            ));
        }
    }
    out.push_str("    return tbl;\n");
    out.push_str("}\n\n");

    // 4. Table destructor
    out.push_str(&format!(
        "static inline void Axon_Table_{0}_free(Axon_Table_{0}* tbl) {{\n",
        struct_name
    ));
    out.push_str("    if (!tbl) return;\n");
    for f_name in &s_info.field_order {
        let field_str = interner.resolve(f_name);
        out.push_str(&format!("    axon_aligned_free(tbl->{});\n", field_str));
    }
    out.push_str("    tbl->len = 0;\n");
    out.push_str("    tbl->capacity = 0;\n");
    out.push_str("}\n\n");

    out
}
