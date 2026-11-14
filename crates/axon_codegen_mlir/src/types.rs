//! MLIR Type Mapping for Axon

use axon_ast::PrimitiveType;
use axon_types::Type;
use lasso::Rodeo;

pub fn to_mlir_type(ty: &Type, interner: &Rodeo) -> String {
    match ty {
        Type::Primitive(p) => match p {
            PrimitiveType::I8 | PrimitiveType::U8 => "i8".to_string(),
            PrimitiveType::I16 | PrimitiveType::U16 => "i16".to_string(),
            PrimitiveType::I32 | PrimitiveType::U32 => "i32".to_string(),
            PrimitiveType::I64 | PrimitiveType::U64 => "i64".to_string(),
            PrimitiveType::F32 => "f32".to_string(),
            PrimitiveType::F64 => "f64".to_string(),
            PrimitiveType::Bool => "i1".to_string(),
            PrimitiveType::Usize => "index".to_string(),
            PrimitiveType::Void => "none".to_string(),
        },
        Type::Struct(s) => format!("!llvm.struct<\"{}\">", interner.resolve(s)),
        Type::Table(s) => format!("!axon.table<\"{}\">", interner.resolve(s)),
        Type::Reference { inner, .. } => format!("!llvm.ptr<{}>", to_mlir_type(inner, interner)),
        Type::Refined { base } => to_mlir_type(base, interner),
        Type::Job(inner) => format!("!axon.job<{}>", to_mlir_type(inner, interner)),
        Type::Unknown => "i64".to_string(),
    }
}

pub fn to_memref_type(scalar_ty: &Type, interner: &Rodeo) -> String {
    let elem = to_mlir_type(scalar_ty, interner);
    format!("memref<?x{}, 64>", elem)
}

pub fn to_vector_type(scalar_ty: &Type, width: usize, interner: &Rodeo) -> String {
    let elem = to_mlir_type(scalar_ty, interner);
    format!("vector<{}x{}>", width, elem)
}
