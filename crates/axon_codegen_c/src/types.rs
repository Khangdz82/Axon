//! Type mapping from Axon types to C23 types

use axon_ast::PrimitiveType;
use axon_types::Type;
use lasso::Rodeo;

pub fn to_c_type(ty: &Type, interner: &Rodeo) -> String {
    match ty {
        Type::Primitive(p) => match p {
            PrimitiveType::I8 => "int8_t".into(),
            PrimitiveType::I16 => "int16_t".into(),
            PrimitiveType::I32 => "int32_t".into(),
            PrimitiveType::I64 => "int64_t".into(),
            PrimitiveType::U8 => "uint8_t".into(),
            PrimitiveType::U16 => "uint16_t".into(),
            PrimitiveType::U32 => "uint32_t".into(),
            PrimitiveType::U64 => "uint64_t".into(),
            PrimitiveType::Usize => "size_t".into(),
            PrimitiveType::F32 => "float".into(),
            PrimitiveType::F64 => "double".into(),
            PrimitiveType::Bool => "bool".into(),
            PrimitiveType::Void => "void".into(),
        },
        Type::Struct(name) => {
            format!("Axon_{}", interner.resolve(name))
        }
        Type::Table(elem) => {
            format!("Axon_Table_{}", interner.resolve(elem))
        }
        Type::Reference { inner, is_mut } => {
            let inner_c = to_c_type(inner, interner);
            if *is_mut {
                format!("{}*", inner_c)
            } else {
                format!("const {}*", inner_c)
            }
        }
        Type::Refined { base } => to_c_type(base, interner),
        Type::Unknown => "int64_t".into(),
    }
}
