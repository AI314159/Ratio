mod emitter;
mod lexer;
mod common;
mod parser;
mod file_io;

use common::{Position, Token};
use lexer::Lexer;
use emitter::CodeGenerator;

use std::process;

use clap::Parser;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    source_path: std::path::PathBuf,

    #[arg(short, long)]
    output: String,

}

fn main() {
    let args = Arguments::parse();
    let input = file_io::read_file(&args.source_path).expect("Failed to read file");
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
    let mut parser = parser::Parser::new(tokens);
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
        .args(&["-static", "/tmp/output.o", "-o", &args.output])
        .status()
        .expect("Failed to execute gcc");
}
