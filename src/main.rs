mod emitter;
mod lexer;
mod common;
mod parser;
mod file_io;

use parser::Parser;
use common::{Position, Token};
use lexer::Lexer;
use emitter::CodeGenerator;

use std::process;
fn main() {
    let input = file_io::read_file("input.ratio").expect("Failed to read file");
    let input = input.trim();

    let mut lexer = Lexer::new(input);
    let mut tokens: Vec<(Token, Position)> = Vec::new();
    loop {
        let (token, pos) = lexer.next_token();
        if token == Token::EOF {
            break;
        }
        tokens.push((token, pos));
    }
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("E: {}", e);
            std::process::exit(1);
        }
    };

    let mut generator = CodeGenerator::new();
    generator.generate(&ast);
    
    file_io::write_file("/tmp/output.asm", &generator.output).expect("Failed to write to file");

    process::Command::new("nasm")
        .args(&["-f", "elf64", "/tmp/output.asm", "-o", "/tmp/output.o"])
        .status()
        .expect("Failed to execute nasm");
    process::Command::new("gcc")
        .args(&["-static", "/tmp/output.o", "-o", "output"])
        .status()
        .expect("Failed to execute gcc");
}
