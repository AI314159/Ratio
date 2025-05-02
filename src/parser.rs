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

        // This code consumes everything between the parentheses. Currently, we discard it.
        // However, we should be taking this info and using it to check function calls.
        let _ = self.parse_function_declaration_arguments()?;
        self.expect(Token::Colon).map_err(|e| CompileError::new(
            format!("Missing colon after function declaration: {}", e.message),
            e.position,
        ))?;
        
        let mut body = Vec::new();
        while !matches!(self.current_token.0, Token::EOF) {
            body.push(self.parse_statement()?);
        }
        
        Ok(Stmt::Function { name, body })
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        match &self.current_token.0 {
            Token::Keyword(Keyword::Var) => self.parse_variable_decl(),
            _ => {
                if (self.peek().0 == Token::Equals) {
                    return self.parse_variable_assignment();
                }
                self.parse_expression_statement()
            },
        }
    }

    fn parse_variable_decl(&mut self) -> Result<Stmt, CompileError> {
        self.expect_keyword(Keyword::Var)?;
        let name = self.parse_identifier()?;
        
        self.expect(Token::Colon)?;
        
        // TODO: more types
        self.expect_keyword(Keyword::Int)?;
        
        self.expect(Token::Equals)?;
        let value = self.parse_expression()?;
        
        Ok(Stmt::VariableDecl { 
            name,
            type_name: "int".to_string(),
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
        match &self.current_token.0 {
            Token::Builtin(builtin) => {
                let callee = match builtin {
                    Builtin::Print => "print",
                    Builtin::Input => "input",
                }.to_string();
                self.advance();
                self.parse_call(callee)
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Variable(name))
            }
            Token::NumberLiteral(n) => {
                let value = *n;
                self.advance();
                Ok(Expr::IntegerLiteral(value))
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringLiteral(s))
            }
            _ => Err(CompileError::new(
                format!("Unexpected token in expression: {:?}", self.current_token.0),
                self.current_token.1.clone(),
            )),
        }
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

    fn peek(&self) -> (Token, Position) {
        if self.index + 1 < self.tokens.len() {
            self.tokens[self.index + 1].clone()
        } else {
            (Token::EOF, Position::new(0, 0))
        }
    }
}