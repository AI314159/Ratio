use crate::common::{Builtin, Keyword, Position, Token};

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    current_pos: Position,
    start_pos: Position,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            position: 0,
            current_pos: Position::new(1, 1),
            start_pos: Position::new(1, 1),
        }
    }

    pub fn next_token(&mut self) -> (Token, Position) {
        if self.position >= self.input.len() {
            return (Token::EOF, self.start_pos.clone());
        }
        self.start_pos = self.current_pos.clone();
        let current = self.current_char();

        // Skip newline
        if current == '\n' {
            self.advance();
            return self.next_token();
        }

        // Skip whitespace within line (not at line start)
        if current == ' ' || current == '\t' || current == '\r' {
            self.advance();
            return self.next_token();
        }

        self.start_pos = self.current_pos.clone();

        if self.position >= self.input.len() {
            return (Token::EOF, self.start_pos);
        }

        let current = self.current_char();
        let token = match current {
            '(' => self.consume_simple(Token::LeftParen),
            ')' => self.consume_simple(Token::RightParen),
            ':' => self.consume_simple(Token::Colon),
            ',' => self.consume_simple(Token::Comma),
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    self.advance();
                    Token::Equality
                } else {
                    self.consume_simple(Token::Equals)
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    self.advance();
                    Token::GreaterThanOrEqual
                } else {
                    self.consume_simple(Token::GreaterThan)
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    self.advance();
                    Token::LessThanOrEqual
                } else {
                    self.consume_simple(Token::LessThan)
                }
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    self.advance();
                    Token::NotEqual
                } else {
                    // TODO
                    panic!("Unary operators are not yet supported");
                }
            }
            '+' => self.consume_simple(Token::Plus),
            '-' => self.consume_simple(Token::Minus),
            '*' => self.consume_simple(Token::Asterisk),
            '/' => self.consume_simple(Token::Slash),
            '"' => self.consume_string(),
            '{' => self.consume_simple(Token::LeftBrace),
            '}' => self.consume_simple(Token::RightBrace),
            ';' => self.consume_simple(Token::Semicolon),
            _ if current.is_alphabetic() || current == '_' => self.consume_word(),
            _ if current.is_digit(10) => self.consume_number(),
            _ => panic!(
                "Unexpected character at {}:{}",
                self.current_pos.line, self.current_pos.column
            ),
        };

        (token, self.start_pos)
    }

    fn consume_number(&mut self) -> Token {
        let start = self.position;
        while self.position < self.input.len() && self.current_char().is_digit(10) {
            self.advance();
        }

        let num_str = &self.input[start..self.position];
        Token::NumberLiteral(num_str.parse().unwrap())
    }

    fn consume_word(&mut self) -> Token {
        let start = self.position;
        while self.position < self.input.len()
            && (self.current_char().is_alphanumeric() || self.current_char() == '_')
        {
            self.advance();
        }

        let word = &self.input[start..self.position];
        match word {
            "fn" => Token::Keyword(Keyword::Fn),
            "extern" => Token::Keyword(Keyword::Extern),
            "var" => Token::Keyword(Keyword::Var),
            "int" => Token::Keyword(Keyword::Int),
            "bool" => Token::Keyword(Keyword::Bool),
            "true" => Token::Keyword(Keyword::True),
            "false" => Token::Keyword(Keyword::False),
            "return" => Token::Keyword(Keyword::Return),
            "print" => Token::Builtin(Builtin::Print),
            "input" => Token::Builtin(Builtin::Input),
            "if" => Token::Keyword(Keyword::If),
            "while" => Token::Keyword(Keyword::While),
            _ => Token::Identifier(word.to_string()),
        }
    }

    fn consume_string(&mut self) -> Token {
        // Skip the first quote
        self.advance();
        let start = self.position;

        while self.position < self.input.len() && self.current_char() != '"' {
            self.advance();
        }

        let content = self.input[start..self.position].to_string();

        // Skip the closing quote
        self.advance();
        Token::StringLiteral(content)
    }

    fn current_char(&self) -> char {
        self.input.chars().nth(self.position).unwrap()
    }

    fn advance(&mut self) {
        if self.current_char() == '\n' {
            self.current_pos.line += 1;
            self.current_pos.column = 1;
        } else {
            self.current_pos.column += 1;
        }
        self.position += 1;
    }

    fn consume_simple(&mut self, token: Token) -> Token {
        self.advance();
        token
    }

    fn peek(&self) -> char {
        if self.position + 1 < self.input.len() {
            self.input.chars().nth(self.position + 1).unwrap()
        } else {
            // TODO: better handling
            '\0'
        }
    }
}
