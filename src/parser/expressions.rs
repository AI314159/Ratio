use crate::{
    common::{Builtin, CompileError, Expr, Keyword, Position, Program, Stmt, Token},
    parser::{Parser, functions},
};

pub fn parse_expression(parser: &mut Parser) -> Result<Expr, CompileError> {
    parse_expression_until(
        parser,
        &[
            Token::LeftBrace,
            Token::RightBrace,
            Token::Semicolon,
            Token::Comma,
            Token::RightParen,
            Token::EOF,
        ],
    )
}

pub fn parse_expression_until(
    parser: &mut Parser,
    stop_tokens: &[Token],
) -> Result<Expr, CompileError> {
    let mut left = match &parser.current_token.0 {
        Token::Builtin(builtin) => {
            let callee = match builtin {
                Builtin::Print => "print",
                Builtin::Input => "input",
            }
            .to_string();
            parser.advance();
            return functions::parse_call(parser, callee);
        }
        Token::Keyword(Keyword::True) => {
            parser.advance();
            Expr::BooleanLiteral(true)
        }
        Token::Keyword(Keyword::False) => {
            parser.advance();
            Expr::BooleanLiteral(false)
        }
        Token::Identifier(name) => {
            let name = name.clone();
            parser.advance();
            if matches!(parser.current_token.0, Token::LeftParen) {
                return functions::parse_call(parser, name);
            }
            Expr::Variable(name)
        }
        Token::NumberLiteral(n) => {
            let value = *n;
            parser.advance();
            Expr::IntegerLiteral(value)
        }
        Token::StringLiteral(s) => {
            let s = s.clone();
            parser.advance();
            Expr::StringLiteral(s)
        }
        _ => {
            eprintln!(
                "DEBUG: Unexpected token in expression: {:?} at {:?}",
                parser.current_token.0, parser.current_token.1
            );
            return Err(CompileError::new(
                format!(
                    "Unexpected token in expression: {:?}",
                    parser.current_token.0
                ),
                parser.current_token.1.clone(),
            ));
        }
    };
    loop {
        match &parser.current_token.0 {
            Token::LeftBrace
            | Token::RightBrace
            | Token::Semicolon
            | Token::Comma
            | Token::RightParen
            | Token::EOF => {
                break;
            }
            Token::Plus | Token::Minus | Token::Asterisk | Token::Slash => {
                let op = parser.current_token.0.clone();
                parser.advance();
                if stop_tokens
                    .iter()
                    .any(|stop| parser.current_token.0 == *stop)
                {
                    eprintln!(
                        "DEBUG: Operator {:?} followed by stop token {:?} at {:?}",
                        op, parser.current_token.0, parser.current_token.1
                    );
                    return Err(CompileError::new(
                        format!(
                            "Expected expression after operator {:?}, found block/statement delimiter",
                            op
                        ),
                        parser.current_token.1.clone(),
                    ));
                }
                let right = parse_expression_until(parser, stop_tokens)?;
                left = Expr::BinaryOperator {
                    operator: parser.get_operator(op),
                    left: Box::new(left),
                    right: Box::new(right),
                };
            }
            Token::Equality
            | Token::GreaterThan
            | Token::LessThan
            | Token::GreaterThanOrEqual
            | Token::LessThanOrEqual
            | Token::NotEqual => {
                let op = parser.current_token.0.clone();
                parser.advance();
                if stop_tokens
                    .iter()
                    .any(|stop| parser.current_token.0 == *stop)
                {
                    eprintln!(
                        "DEBUG: Comparison operator {:?} followed by stop token {:?} at {:?}",
                        op, parser.current_token.0, parser.current_token.1
                    );
                    return Err(CompileError::new(
                        format!(
                            "Expected expression after operator {:?}, found block/statement delimiter",
                            op
                        ),
                        parser.current_token.1.clone(),
                    ));
                }
                let right = parse_expression_until(parser, stop_tokens)?;
                left = Expr::BooleanComparison {
                    lvalue: Box::new(left),
                    operator: op,
                    rvalue: Box::new(right),
                };
            }
            _ => break,
        }
    }
    Ok(left)
}
