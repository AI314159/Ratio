use super::Parser;
use crate::{
    common::{CompileError, Expr, ExternFunction, Keyword, Stmt, Token, Type},
    parser::expressions,
    type_system::keyword_to_type,
};

pub fn parse_extern_function(parser: &mut Parser) -> Result<ExternFunction, CompileError> {
    parser.expect_keyword(Keyword::Extern)?;
    parser.expect_keyword(Keyword::Fn)?;
    let name = parser.parse_identifier()?;
    let args = parse_extern_function_args(parser)?;
    let return_type = if let Token::Identifier(ref t) = parser.current_token.0 {
        let t = t.clone();
        parser.advance();
        t
    } else if matches!(parser.current_token.0, Token::Semicolon) {
        // No return type specified, treat as void
        String::new()
    } else {
        return Err(CompileError::new(
            "Expected return type after extern fn args or ';'",
            parser.current_token.1.clone(),
        ));
    };
    parser.expect(Token::Semicolon)?;
    Ok(ExternFunction {
        name,
        args,
        return_type,
    })
}

pub fn parse_extern_function_args(
    parser: &mut Parser,
) -> Result<Vec<(String, Type)>, CompileError> {
    parser.expect(Token::LeftParen)?;
    let mut args = Vec::new();
    while !matches!(parser.current_token.0, Token::RightParen) {
        let name = parser.parse_identifier()?;
        parser.expect(Token::Colon)?;
        let t = if let Token::Keyword(kw) = &parser.current_token.0 {
            keyword_to_type(kw, &parser.current_token.1)?
        } else {
            return Err(CompileError::new(
                "Expected type in extern fn arg",
                parser.current_token.1.clone(),
            ));
        };
        parser.advance();
        args.push((name, t));
        if matches!(parser.current_token.0, Token::Comma) {
            parser.advance();
        }
    }
    parser.expect(Token::RightParen)?;
    Ok(args)
}

pub fn parse_function(parser: &mut Parser) -> Result<Stmt, CompileError> {
    parser.expect_keyword(Keyword::Fn)?;
    let name = parser.parse_identifier()?;
    let args = parse_function_declaration_arguments_with_types(parser)?;
    let body = parser.parse_block()?;

    let mut return_expr = None;
    if matches!(parser.current_token.0, Token::Keyword(Keyword::Return)) {
        parser.advance();
        return_expr = Some(expressions::parse_expression(parser)?);
        if matches!(parser.current_token.0, Token::Semicolon) {
            parser.advance();
        }
    }
    Ok(Stmt::Function {
        name,
        args,
        body,
        return_expr,
    })
}

pub fn parse_function_declaration_arguments_with_types(
    parser: &mut Parser,
) -> Result<Vec<(String, Type)>, CompileError> {
    parser.expect(Token::LeftParen)?;
    let mut args = Vec::new();
    if matches!(parser.current_token.0, Token::RightParen) {
        parser.expect(Token::RightParen)?;
        return Ok(args);
    }
    while !matches!(parser.current_token.0, Token::RightParen) {
        let name = parser.parse_identifier()?;
        parser.expect(Token::Colon)?;

        let t = if let Token::Keyword(kw) = &parser.current_token.0 {
            keyword_to_type(kw, &parser.current_token.1)?
        } else {
            return Err(CompileError::new(
                "Expected type in fn arg",
                parser.current_token.1.clone(),
            ));
        };
        parser.advance();
        args.push((name, t));
        if matches!(parser.current_token.0, Token::Comma) {
            parser.advance();
        }
    }
    parser.expect(Token::RightParen)?;
    Ok(args)
}

pub fn parse_call(parser: &mut Parser, callee: String) -> Result<Expr, CompileError> {
    parser.expect(Token::LeftParen)?;
    let mut args = Vec::new();

    while !matches!(parser.current_token.0, Token::RightParen) {
        args.push(expressions::parse_expression(parser)?);
        if matches!(parser.current_token.0, Token::Comma) {
            parser.advance();
        }
    }

    parser.expect(Token::RightParen)?;
    Ok(Expr::Call { callee, args })
}
