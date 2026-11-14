//! Type mapping between Axon types and Cranelift types

use axon_ast::PrimitiveType;
use axon_types::Type;
use cranelift::prelude::types;

pub fn map_type(ty: &Type, ptr_type: types::Type) -> Option<types::Type> {
    match ty {
        Type::Primitive(p) => match p {
            PrimitiveType::I8 | PrimitiveType::U8 | PrimitiveType::Bool => Some(types::I8),
            PrimitiveType::I16 | PrimitiveType::U16 => Some(types::I16),
            PrimitiveType::I32 | PrimitiveType::U32 => Some(types::I32),
            PrimitiveType::I64 | PrimitiveType::U64 => Some(types::I64),
            PrimitiveType::F32 => Some(types::F32),
            PrimitiveType::F64 => Some(types::F64),
            PrimitiveType::Usize => Some(ptr_type),
            PrimitiveType::Void => None,
        },
        Type::Struct(_) | Type::Table(_) | Type::Reference { .. } | Type::Job(_) => Some(ptr_type),
        Type::Refined { base } => map_type(base, ptr_type),
        Type::Unknown => Some(types::I64),
    }
}
