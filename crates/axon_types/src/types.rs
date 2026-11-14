//! Type representation for Axon

use axon_ast::PrimitiveType;
use lasso::Spur;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(Spur),
    Table(Spur),
    Reference {
        inner: Box<Type>,
        is_mut: bool,
    },
    Refined {
        base: Box<Type>,
    },
    Unknown,
}

impl Type {
    pub const VOID: Type = Type::Primitive(PrimitiveType::Void);
    pub const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    pub const I32: Type = Type::Primitive(PrimitiveType::I32);
    pub const I64: Type = Type::Primitive(PrimitiveType::I64);
    pub const F32: Type = Type::Primitive(PrimitiveType::F32);
    pub const F64: Type = Type::Primitive(PrimitiveType::F64);
    pub const USIZE: Type = Type::Primitive(PrimitiveType::Usize);

    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Reference { .. })
    }

    pub fn is_table(&self) -> bool {
        matches!(self, Type::Table(_))
    }

    pub fn is_numeric(&self) -> bool {
        match self {
            Type::Primitive(p) => match p {
                PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::Usize
                | PrimitiveType::F32
                | PrimitiveType::F64 => true,
                _ => false,
            },
            _ => false,
        }
    }
}
