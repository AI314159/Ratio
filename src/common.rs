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
    Extern,
    Var,
    Return,
    Int,
    Bool,
    True,
    False,

    If,
    Else,
    While,
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
    Plus,
    Minus,
    Asterisk,
    Slash,
    Equality,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    StringLiteral(String),
    NumberLiteral(i64),
    Identifier(String),
    EOF,
    LeftBrace,
    RightBrace,
    Semicolon,
}
impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            match self {
                // TODO: Find a better way to do this
                Token::Keyword(k) => format!("{:?}", k),
                Token::Builtin(b) => format!("{:?}", b),
                Token::LeftParen => "(".to_string(),
                Token::RightParen => ")".to_string(),
                Token::Colon => ":".to_string(),
                Token::Comma => ",".to_string(),
                Token::Equals => "=".to_string(),
                Token::Plus => "+".to_string(),
                Token::Minus => "-".to_string(),
                Token::Asterisk => "*".to_string(),
                Token::Slash => "/".to_string(),
                Token::Equality => "==".to_string(),
                Token::LessThan => "<".to_string(),
                Token::LessThanOrEqual => "<=".to_string(),
                Token::GreaterThan => ">".to_string(),
                Token::GreaterThanOrEqual => ">=".to_string(),
                Token::NotEqual => "!=".to_string(),
                Token::StringLiteral(s) => format!("\"{}\"", s),
                Token::NumberLiteral(n) => n.to_string(),
                Token::Identifier(s) => s.clone(),
                Token::EOF => "EOF".to_string(),
                Token::LeftBrace => "{".to_string(),
                Token::RightBrace => "}".to_string(),
                Token::Semicolon => ";".to_string(),
            }
        )
    }
}
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Stmt>,
    pub externs: Vec<ExternFunction>,
}

#[derive(Debug, Clone)]
pub struct ExternFunction {
    pub name: String,
    pub args: Vec<(String, String)>, // (name, type)
    pub return_type: String,
}

#[derive(Debug)]
pub enum Stmt {
    Function {
        name: String,
        args: Vec<(String, String)>,
        body: Vec<Stmt>,
        return_expr: Option<Expr>,
    },
    Return(Expr),
    ExternFunction(ExternFunction),
    VariableDecl {
        name: String,
        type_name: String,
        value: Expr,
    },
    Assignment {
        name: String,
        value: Expr,
    },

    IfStatement {
        condition: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
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
    BinaryOperator {
        operator: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    BooleanComparison {
        lvalue: Box<Expr>,
        operator: Token,
        rvalue: Box<Expr>,
    },
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