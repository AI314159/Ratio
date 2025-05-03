use crate::common::{Position, Token, Keyword, Builtin};

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    current_pos: Position,
    start_pos: Position,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
    at_line_start: bool,

}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input,
            position: 0,
            current_pos: Position::new(1, 1),
            start_pos: Position::new(1, 1),
            indent_stack: vec![0],
            pending_dedents: 0,
            at_line_start: true,
        }
    }

    pub fn next_token(&mut self) -> (Token, Position) {
        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            return (Token::Dedent, self.start_pos.clone());
        }

        if self.position >= self.input.len() {
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                return (Token::Dedent, self.start_pos.clone());
            }
            return (Token::EOF, self.start_pos.clone());
        }
        // Handle start of line: measure indentation
        if self.at_line_start {
            let indent = self.consume_indent();
            let last_indent = *self.indent_stack.last().unwrap();
            if indent > last_indent {
                self.indent_stack.push(indent);
                self.at_line_start = false;
                return (Token::Indent, self.start_pos.clone());
            } else if indent < last_indent {
                self.pending_dedents = 0;
                while indent < *self.indent_stack.last().unwrap() {
                    self.indent_stack.pop();
                    self.pending_dedents += 1;
                }
                if indent != *self.indent_stack.last().unwrap() {
                    panic!("Inconsistent indentation at line {}", self.current_pos.line);
                }
                if self.pending_dedents > 0 {
                    self.pending_dedents -= 1;
                    return (Token::Dedent, self.start_pos.clone());
                }
            }
            self.at_line_start = false;
        }

        self.start_pos = self.current_pos.clone();

        let current = self.current_char();

        // Handle newline
        if current == '\n' {
            self.advance();
            self.at_line_start = true;
            return (Token::Newline, self.start_pos.clone());
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
            '=' => self.consume_simple(Token::Equals),
            '+' => self.consume_simple(Token::Plus),
            '-' => self.consume_simple(Token::Minus),
            '*' => self.consume_simple(Token::Asterisk),
            '/' => self.consume_simple(Token::Slash),
            '"' => self.consume_string(),
            _ if current.is_alphabetic() => self.consume_word(),
            _ if current.is_digit(10) => self.consume_number(),  // Add this
            _ => panic!("Unexpected character at {}:{}", self.current_pos.line, self.current_pos.column),
        };

        (token, self.start_pos)
    }

    fn consume_indent(&mut self) -> usize {
        let mut count = 0;
        while self.position < self.input.len() {
            match self.current_char() {
                ' ' => {
                    count += 1;
                    self.advance();
                }
                '\t' => {
                    count += 4; // or whatever your tab width is
                    self.advance();
                }
                _ => break,
            }
        }
        count
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
        while self.position < self.input.len() && self.current_char().is_alphanumeric() {
            self.advance();
        }

        let word = &self.input[start..self.position];
        match word {
            "fn" => Token::Keyword(Keyword::Fn),
            "var" => Token::Keyword(Keyword::Var),
            "int" => Token::Keyword(Keyword::Int),
            "bool" => Token::Keyword(Keyword::Bool),
            "true" => Token::Keyword(Keyword::True),
            "false" => Token::Keyword(Keyword::False),
            "return" => Token::Keyword(Keyword::Return),
            "print" => Token::Builtin(Builtin::Print),
            "input" => Token::Builtin(Builtin::Input),
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

    fn skip_whitespace(&mut self) {
        // TODO: this should definitely be tokens.
        while self.position < self.input.len() {
            match self.current_char() {
                ' ' | '\t' | '\r' => self.advance(),
                '\n' => {
                    self.advance();
                    self.start_pos = self.current_pos.clone();
                }
                _ => break,
            }
        }
    }

    fn consume_simple(&mut self, token: Token) -> Token {
        self.advance();
        token
    }
}