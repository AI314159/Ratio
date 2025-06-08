use crate::{
    common::{CompileError, Keyword, Stmt, Token},
    parser::{Parser, expressions},
};

pub fn parse_if_statement(parser: &mut Parser) -> Result<Stmt, CompileError> {
    parser.expect_keyword(Keyword::If)?;
    let condition = expressions::parse_expression_until(
        parser,
        &[
            Token::LeftBrace,
            Token::RightBrace,
            Token::Semicolon,
            Token::Comma,
            Token::RightParen,
            Token::EOF,
        ],
    )?;
    let body = parser.parse_block()?;
    let mut else_body = None;
    if matches!(parser.current_token.0, Token::Keyword(Keyword::Else)) {
        parser.advance();
        if matches!(parser.current_token.0, Token::Keyword(Keyword::If)) {
            // Recursively parse chained else if
            let else_if_stmt = parse_if_statement(parser)?;
            else_body = Some(vec![else_if_stmt]);
        } else if matches!(parser.current_token.0, Token::LeftBrace) {
            // Only parse a block after else, not an expression
            else_body = Some(parser.parse_block()?);
        } else {
            return Err(CompileError::new(
                "Expected '{' after 'else'",
                parser.current_token.1.clone(),
            ));
        }
    }
    Ok(Stmt::IfStatement {
        condition,
        body,
        else_body,
    })
}

pub fn parse_while_statement(parser: &mut Parser) -> Result<Stmt, CompileError> {
    parser.expect_keyword(Keyword::While)?;
    let condition = expressions::parse_expression_until(
        parser,
        &[
            Token::LeftBrace,
            Token::RightBrace,
            Token::Semicolon,
            Token::Comma,
            Token::RightParen,
            Token::EOF,
        ],
    )?;
    let body = parser.parse_block()?;
    Ok(Stmt::While { condition, body })
}
