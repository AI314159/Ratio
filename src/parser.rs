use crate::common::{CompileError, Position, Token, Keyword, Builtin, Stmt, Expr};

pub struct Parser {
    tokens: Vec<(Token, Position)>,
    current_token: (Token, Position),
    index: usize,

}

impl Parser {
    pub fn new(tokens: Vec<(Token, Position)>) -> Self {
        let current_token = tokens[0].clone();
        Self { tokens, current_token, index: 0 }
    }

    pub fn parse(&mut self) -> Result<Stmt, CompileError> {
        self.parse_function()
    }

    fn parse_function(&mut self) -> Result<Stmt, CompileError> {
        self.expect_keyword(Keyword::Fn)?;
        let name = self.parse_identifier()?;
        let _ = self.parse_function_declaration_arguments()?;
        let body = self.parse_block()?;
        Ok(Stmt::Function { name, body })
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
            Token::Keyword(Keyword::Var) => self.parse_variable_decl(),
            Token::Keyword(Keyword::If) => self.parse_if_statement(),
            Token::Keyword(Keyword::While) => self.parse_while_statement(),
            Token::LeftBrace => {
                // Standalone blocks; is it supported in the AST?
                self.parse_block()?;
                // THIS SHOULD NOT BE RETURNED IF STANDALONE BLOCKS ARE TO WORK
                Ok(Stmt::ExprStmt(Expr::BooleanLiteral(true)))
            }
            Token::RightBrace | Token::EOF => {
                Err(CompileError::new(
                    format!("Unexpected block delimiter or EOF in statement context: {:?}", self.current_token.0),
                    self.current_token.1.clone(),
                ))
            }
            _ => {
                if self.peek().0 == Token::Equals {
                    return self.parse_variable_assignment();
                }
                self.parse_expression_statement()
            },
        }
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, CompileError> {
        self.expect_keyword(Keyword::While)?;
        let condition = self.parse_expression_until(&[
            Token::LeftBrace, Token::RightBrace, Token::Semicolon, Token::Comma, Token::RightParen, Token::EOF
        ])?;
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_variable_decl(&mut self) -> Result<Stmt, CompileError> {
        self.expect_keyword(Keyword::Var)?;
        let name = self.parse_identifier()?;
        self.expect(Token::Colon)?;
        let type_name = match self.current_token.0 {
            Token::Keyword(Keyword::Int) => "int",
            Token::Keyword(Keyword::Bool) => "bool",
            _ => return Err(CompileError::new(
                "Expected known type after variable declaration",
                self.current_token.1.clone(),
            )),
        };
        self.advance();
        self.expect(Token::Equals)?;
        let value = self.parse_expression()?;
        Ok(Stmt::VariableDecl { 
            name,
            type_name: type_name.to_string(),
            value
        })
    }

    fn parse_variable_assignment(&mut self) -> Result<Stmt, CompileError> {
        let name = self.parse_identifier()?;
        self.expect(Token::Equals)?;
        let value = self.parse_expression()?;
        Ok(Stmt::Assignment { name, value })
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt, CompileError> {
        let expr = self.parse_expression()?;
        Ok(Stmt::ExprStmt(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr, CompileError> {
        self.parse_expression_until(&[Token::LeftBrace, Token::RightBrace, Token::Semicolon, Token::Comma, Token::RightParen, Token::EOF])
    }

    fn parse_expression_until(&mut self, stop_tokens: &[Token]) -> Result<Expr, CompileError> {
        let mut left = match &self.current_token.0 {
            Token::Builtin(builtin) => {
                let callee = match builtin {
                    Builtin::Print => "print",
                    Builtin::Input => "input",
                }.to_string();
                self.advance();
                return self.parse_call(callee);
            }
            Token::Keyword(Keyword::True) => {
                self.advance();
                Expr::BooleanLiteral(true)
            }
            Token::Keyword(Keyword::False) => {
                self.advance();
                Expr::BooleanLiteral(false)
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Expr::Variable(name)
            }
            Token::NumberLiteral(n) => {
                let value = *n;
                self.advance();
                Expr::IntegerLiteral(value)
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Expr::StringLiteral(s)
            }
            _ => {
                eprintln!("DEBUG: Unexpected token in expression: {:?} at {:?}", self.current_token.0, self.current_token.1);
                return Err(CompileError::new(
                    format!("Unexpected token in expression: {:?}", self.current_token.0),
                    self.current_token.1.clone(),
                ));
            },
        };
        loop {
            match &self.current_token.0 {
                Token::LeftBrace | Token::RightBrace | Token::Semicolon | Token::Comma | Token::RightParen | Token::EOF => {
                    break;
                }
                Token::Plus | Token::Minus | Token::Asterisk | Token::Slash => {
                    let op = self.current_token.0.clone();
                    self.advance();
                    if stop_tokens.iter().any(|stop| self.current_token.0 == *stop) {
                        eprintln!("DEBUG: Operator {:?} followed by stop token {:?} at {:?}", op, self.current_token.0, self.current_token.1);
                        return Err(CompileError::new(
                            format!("Expected expression after operator {:?}, found block/statement delimiter", op),
                            self.current_token.1.clone(),
                        ));
                    }
                    let right = self.parse_expression_until(stop_tokens)?;
                    left = Expr::BinaryOperator {
                        operator: self.get_operator(op),
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                Token::Equality | Token::GreaterThan | Token::LessThan | Token::GreaterThanOrEqual | Token::LessThanOrEqual | Token::NotEqual => {
                    let op = self.current_token.0.clone();
                    self.advance();
                    if stop_tokens.iter().any(|stop| self.current_token.0 == *stop) {
                        eprintln!("DEBUG: Comparison operator {:?} followed by stop token {:?} at {:?}", op, self.current_token.0, self.current_token.1);
                        return Err(CompileError::new(
                            format!("Expected expression after operator {:?}, found block/statement delimiter", op),
                            self.current_token.1.clone(),
                        ));
                    }
                    let right = self.parse_expression_until(stop_tokens)?;
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

    fn parse_call(&mut self, callee: String) -> Result<Expr, CompileError> {
        self.expect(Token::LeftParen)?;
        let mut args = Vec::new();
        
        while !matches!(self.current_token.0, Token::RightParen) {
            args.push(self.parse_expression()?);
            if matches!(self.current_token.0, Token::Comma) {
                self.advance();
            }
        }
        
        self.expect(Token::RightParen)?;
        Ok(Expr::Call { callee, args })
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, CompileError> {
        self.expect_keyword(Keyword::If)?;
        let condition = self.parse_expression_until(&[
            Token::LeftBrace, Token::RightBrace, Token::Semicolon, Token::Comma, Token::RightParen, Token::EOF
        ])?;
        let body = self.parse_block()?;
        let mut else_body = None;
        if matches!(self.current_token.0, Token::Keyword(Keyword::Else)) {
            self.advance();
            if matches!(self.current_token.0, Token::Keyword(Keyword::If)) {
                // Recursively parse chained else if
                let else_if_stmt = self.parse_if_statement()?;
                else_body = Some(vec![else_if_stmt]);
            } else if matches!(self.current_token.0, Token::LeftBrace) {
                // Only parse a block after else, not an expression
                else_body = Some(self.parse_block()?);
            } else {
                return Err(CompileError::new(
                    "Expected '{' after 'else'",
                    self.current_token.1.clone(),
                ));
            }
        }
        Ok(Stmt::IfStatement { condition, body, else_body })
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

    fn parse_function_declaration_arguments(&mut self) -> Result<Vec<String>, CompileError> {
        // TODO: type checking
        self.expect(Token::LeftParen)?;
        let mut args = Vec::new();
        
        while !matches!(self.current_token.0, Token::RightParen) {
            if let Token::Identifier(name) = &self.current_token.0 {
                args.push(name.clone());
                self.advance();
            } else {
                return Err(CompileError::new(
                    "Expected identifier in function arguments",
                    self.current_token.1.clone(),
                ));
            }
            
            if matches!(self.current_token.0, Token::Comma) {
                self.advance();
            }
        }
        
        self.expect(Token::RightParen)?;
        Ok(args)
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
        let rvalue = self.parse_expression()?;
        Ok(Expr::BinaryOperator {
            operator: self.get_operator(token),
            left: Box::new(Expr::IntegerLiteral(lvalue)),
            right: Box::new(rvalue),
        })
    }

    fn parse_boolean_expression(&mut self, token: Token, lvalue: i64) -> Result<Expr, CompileError> {
        self.advance();
        self.expect(token.clone())?;
        let rvalue = self.parse_expression()?;
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