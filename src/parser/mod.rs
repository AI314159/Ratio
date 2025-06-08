use crate::common::{Builtin, CompileError, Expr, Keyword, Position, Program, Stmt, Token};

pub mod control_flow;
pub mod expressions;
pub mod functions;
pub mod variables;

pub struct Parser {
    tokens: Vec<(Token, Position)>,
    current_token: (Token, Position),
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, Position)>) -> Self {
        let current_token = tokens[0].clone();
        Self {
            tokens,
            current_token,
            index: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, CompileError> {
        let mut functions = Vec::new();
        let mut externs = Vec::new();
        while self.current_token.0 != Token::EOF {
            match &self.current_token.0 {
                Token::Keyword(Keyword::Extern) => {
                    externs.push(functions::parse_extern_function(self)?);
                }
                Token::Keyword(Keyword::Fn) => {
                    functions.push(functions::parse_function(self)?);
                }
                Token::EOF => break,
                _ => {
                    return Err(CompileError::new(
                        format!("Unexpected token at top level: {:?}", self.current_token.0),
                        self.current_token.1.clone(),
                    ));
                }
            }
        }
        Ok(Program { functions, externs })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut body = Vec::new();
        self.expect(Token::LeftBrace)?;
        while !matches!(self.current_token.0, Token::RightBrace | Token::EOF) {
            let stmt = self.parse_statement()?;
            if matches!(self.current_token.0, Token::Semicolon) {
                self.advance();
            }
            body.push(stmt);
        }
        self.expect(Token::RightBrace)?;
        Ok(body)
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        match &self.current_token.0 {
            Token::Keyword(Keyword::Var) => variables::parse_variable_decl(self),
            Token::Keyword(Keyword::If) => control_flow::parse_if_statement(self),
            Token::Keyword(Keyword::While) => control_flow::parse_while_statement(self),
            Token::LeftBrace => {
                // Standalone blocks; is it supported in the AST?
                self.parse_block()?;
                // THIS SHOULD NOT BE RETURNED IF STANDALONE BLOCKS ARE TO WORK
                Ok(Stmt::ExprStmt(Expr::BooleanLiteral(true)))
            }
            Token::RightBrace | Token::EOF => Err(CompileError::new(
                format!(
                    "Unexpected block delimiter or EOF in statement context: {:?}",
                    self.current_token.0
                ),
                self.current_token.1.clone(),
            )),
            Token::Keyword(Keyword::Return) => {
                self.advance();
                let expr = expressions::parse_expression(self)?;
                if matches!(self.current_token.0, Token::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Return(expr))
            }
            _ => {
                if self.peek().0 == Token::Equals {
                    return variables::parse_variable_assignment(self);
                }
                self.parse_expression_statement()
            }
        }
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt, CompileError> {
        let expr = expressions::parse_expression(self)?;
        Ok(Stmt::ExprStmt(expr))
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), CompileError> {
        if let Token::Keyword(k) = &self.current_token.0 {
            if k == &keyword {
                self.advance();
                return Ok(());
            }
        }
        Err(CompileError::new(
            format!("Expected keyword {:?}", keyword),
            self.current_token.1.clone(),
        ))
    }

    fn expect(&mut self, expected: Token) -> Result<(), CompileError> {
        if std::mem::discriminant(&self.current_token.0) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(CompileError::new(
                format!("Expected {:?}, found {:?}", expected, self.current_token.0),
                self.current_token.1.clone(),
            ))
        }
    }

    fn parse_identifier(&mut self) -> Result<String, CompileError> {
        if let Token::Identifier(name) = &self.current_token.0 {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(CompileError::new(
                "Expected identifier",
                self.current_token.1.clone(),
            ))
        }
    }

    fn advance(&mut self) {
        self.index += 1;
        if self.index >= self.tokens.len() {
            self.current_token = (Token::EOF, Position::new(0, 0));
            return;
        }
        self.current_token = self.tokens[self.index].clone();
    }

    fn parse_binary_operator(&mut self, token: Token, lvalue: i64) -> Result<Expr, CompileError> {
        // Note that this expects that the next token is a binary operator, and that the current
        // token is a number literal.
        self.advance();
        self.expect(token.clone())?;
        let rvalue = expressions::parse_expression(self)?;
        Ok(Expr::BinaryOperator {
            operator: self.get_operator(token),
            left: Box::new(Expr::IntegerLiteral(lvalue)),
            right: Box::new(rvalue),
        })
    }

    fn parse_boolean_expression(
        &mut self,
        token: Token,
        lvalue: i64,
    ) -> Result<Expr, CompileError> {
        self.advance();
        self.expect(token.clone())?;
        let rvalue = expressions::parse_expression(self)?;
        Ok(Expr::BooleanComparison {
            lvalue: Box::new(Expr::IntegerLiteral(lvalue)),
            operator: token,
            rvalue: Box::new(rvalue),
        })
    }

    fn get_operator(&self, token: Token) -> String {
        match token {
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::Asterisk => "*".to_string(),
            Token::Slash => "/".to_string(),
            _ => panic!("Unexpected token for binary operator: {:?}", token),
        }
    }

    fn peek(&self) -> (Token, Position) {
        if self.index + 1 < self.tokens.len() {
            self.tokens[self.index + 1].clone()
        } else {
            (Token::EOF, Position::new(0, 0))
        }
    }
}
