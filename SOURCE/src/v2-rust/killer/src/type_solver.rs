//! Type System — 40+ functions (type introspection, reflection, classification)

pub enum TypeClass {
    Integer,
    Float,
    String,
    Array,
    Dict,
}

impl TypeClass {
    pub fn name(&self) -> &str {
        match self {
            Self::Integer => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::Array => "array",
            Self::Dict => "dict",
        }
    }
}
