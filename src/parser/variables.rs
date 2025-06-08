use crate::{
    common::{CompileError, Keyword, Stmt, Token},
    parser::{Parser, expressions},
};

pub fn parse_variable_decl(parser: &mut Parser) -> Result<Stmt, CompileError> {
    parser.expect_keyword(Keyword::Var)?;
    let name = parser.parse_identifier()?;
    parser.expect(Token::Colon)?;
    let type_name = match parser.current_token.0 {
        Token::Keyword(Keyword::Int) => "int",
        Token::Keyword(Keyword::Bool) => "bool",
        _ => {
            return Err(CompileError::new(
                "Expected known type after variable declaration",
                parser.current_token.1.clone(),
            ));
        }
    };
    parser.advance();
    parser.expect(Token::Equals)?;
    let value = expressions::parse_expression(parser)?;
    Ok(Stmt::VariableDecl {
        name,
        type_name: type_name.to_string(),
        value,
    })
}

pub fn parse_variable_assignment(parser: &mut Parser) -> Result<Stmt, CompileError> {
    let name = parser.parse_identifier()?;
    parser.expect(Token::Equals)?;
    let value = expressions::parse_expression(parser)?;
    Ok(Stmt::Assignment { name, value })
}
