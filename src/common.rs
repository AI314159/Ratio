use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Keyword {
    Fn,
    Var,
    Return,
    Int,
    Bool,
    True,
    False,
}

#[derive(Debug, PartialEq)]
pub enum Type {
    Int,
    Bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Builtin {
    // I should find a better way to track builtin functions.
    // Probably something that is implicitly imported at the top of the code file.
    // This contains declarations for builtin functions that are linked in during the linking.
    Print,
    Input,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(Keyword),
    Builtin(Builtin),
    LeftParen,
    RightParen,
    Colon,
    Comma,
    Equals,
    StringLiteral(String),
    NumberLiteral(i64),
    Identifier(String),
    EOF,
}
impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            match self {
                Token::Keyword(k) => format!("{:?}", k),
                Token::Builtin(b) => format!("{:?}", b),
                Token::LeftParen => "(".to_string(),
                Token::RightParen => ")".to_string(),
                Token::Colon => ":".to_string(),
                Token::Comma => ",".to_string(),
                Token::Equals => "=".to_string(),
                Token::StringLiteral(s) => format!("\"{}\"", s),
                Token::NumberLiteral(n) => n.to_string(),
                Token::Identifier(s) => s.clone(),
                Token::EOF => "EOF".to_string(),
            }
        )
    }
}
#[derive(Debug)]
pub enum Stmt {
    Function {
        name: String,
        body: Vec<Stmt>,
    },
    VariableDecl {
        name: String,
        type_name: String,
        value: Expr,
    },
    Assignment {
        name: String,
        value: Expr,
    },
    ExprStmt(Expr),
}

#[derive(Debug)]
pub enum Expr {
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    Variable(String),
    StringLiteral(String),
    IntegerLiteral(i64),
    BooleanLiteral(bool),
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub position: Position,
}

impl CompileError {
    pub fn new(message: impl Into<String>, position: Position) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Error at {}:{}\n{}",
            self.position.line,
            self.position.column,
            self.message,
        )
    }
}

impl Error for CompileError {}