//! Memory layout and byte-size calculations for Axon types.

use axon_ast::PrimitiveType;
use axon_types::{GlobalContext, Type};

pub fn compute_type_size(ty: &Type, ctx: &GlobalContext) -> usize {
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
        Type::Struct(name) => {
            if let Some(s_info) = ctx.structs.get(name) {
                let mut total = 0;
                for field_name in &s_info.field_order {
                    if let Some((field_ty, _)) = s_info.fields.get(field_name) {
                        total += compute_type_size(field_ty, ctx);
                    }
                }
                total
            } else {
                8
            }
        }
        Type::Table(_) => 24, // Pointer (8) + Len (8) + Cap (8)
        Type::Reference { .. } => 8,
        Type::Refined { base } => compute_type_size(base, ctx),
        Type::Unknown => 8,
    }
}
