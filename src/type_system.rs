use crate::common::{CompileError, Keyword, Position, Type};

pub fn keyword_to_type(kw: &Keyword, pos: &Position) -> Result<Type, CompileError> {
    match kw {
        Keyword::Int => Ok(Type::Int),
        Keyword::Bool => Ok(Type::Bool),

        _ => return Err(CompileError::new("Unknown type found", *pos)),
    }
}
