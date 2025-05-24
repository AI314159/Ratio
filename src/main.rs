mod lexer;
mod common;
mod parser;
mod file_io;
mod llvm_codegen;

use common::{Position, Token};
use lexer::Lexer;
// use emitter::CodeGenerator;

use std::process;
use clap::Parser;
use inkwell::context::Context;
use std::fs;


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

    let context = Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();
    llvm_codegen::generate_module(&context, &module, &builder, &ast);

    let llvm_ir = module.print_to_string().to_string();
    let ir_path = "/tmp/output.ll";
    fs::write(ir_path, &llvm_ir).expect("Failed to write LLVM IR");

    let llc_status = process::Command::new("llc")
        .args(["-filetype=obj", ir_path, "-o", "/tmp/output.o"])
        .status()
        .expect("Failed to execute llc");
    if !llc_status.success() {
        eprintln!("llc failed");
        std::process::exit(1);
    }

    let gcc_status = process::Command::new("gcc")
        .args(["-static", "/tmp/output.o", "-o", &args.output])
        .status()
        .expect("Failed to execute gcc");
    if !gcc_status.success() {
        eprintln!("gcc failed");
        std::process::exit(1);
    }
}
