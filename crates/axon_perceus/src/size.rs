//! Memory layout and byte-size calculations for Axon types.

use axon_ast::PrimitiveType;
use axon_types::{GlobalContext, Type};

/// Compute the byte size of an allocated heap or composite object (Struct, Table).
/// Primitive scalar types return 0 as they do not require heap drop/reuse tokens.
pub fn compute_type_size(ty: &Type, ctx: &GlobalContext) -> usize {
    match ty {
        Type::Primitive(_) => 0, // Primitives are scalar values, not heap allocations
        Type::Struct(name) => {
            if let Some(s_info) = ctx.structs.get(name) {
                let mut total = 0;
                for field_name in &s_info.field_order {
                    if let Some((field_ty, _)) = s_info.fields.get(field_name) {
                        total += compute_scalar_size(field_ty, ctx);
                    }
                }
                total
            } else {
                0
            }
        }
        Type::Table(_) => 24, // Pointer (8) + Len (8) + Cap (8)
        Type::Reference { .. } => 0,
        Type::Refined { base } => compute_type_size(base, ctx),
        Type::Job(_) => 8,
        Type::Unknown => 0,
    }
}

/// Compute scalar storage size for fields inside a struct or container.
pub fn compute_scalar_size(ty: &Type, ctx: &GlobalContext) -> usize {
    match ty {
        Type::Primitive(p) => match p {
            PrimitiveType::I8 | PrimitiveType::U8 | PrimitiveType::Bool => 1,
            PrimitiveType::I16 | PrimitiveType::U16 => 2,
            PrimitiveType::I32 | PrimitiveType::U32 | PrimitiveType::F32 => 4,
            PrimitiveType::I64 | PrimitiveType::U64 | PrimitiveType::F64 | PrimitiveType::Usize => {
                8
            }
            PrimitiveType::Void => 0,
        },
        Type::Struct(_) => compute_type_size(ty, ctx),
        Type::Table(_) => 24,
        Type::Reference { .. } => 8,
        Type::Refined { base } => compute_scalar_size(base, ctx),
        Type::Job(_) => 8,
        Type::Unknown => 8,
    }
}
