mod lexer;
mod common;
mod parser;
mod file_io;
mod llvm_codegen;

use common::{Position, Token};
use lexer::Lexer;

use std::process;
use clap::Parser;
use inkwell::context::Context;


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

    inkwell::targets::Target::initialize_all(&inkwell::targets::InitializationConfig::default());
    let target_triple = inkwell::targets::TargetMachine::get_default_triple();
    let target = inkwell::targets::Target::from_triple(&target_triple).expect("Failed to get target");
    let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            inkwell::OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .expect("Failed to create target machine");
    module.set_triple(&target_triple);
    module.set_data_layout(&target_machine.get_target_data().get_data_layout());

    let obj_path = "/tmp/output.o";
    target_machine
        .write_to_file(&module, inkwell::targets::FileType::Object, std::path::Path::new(obj_path))
        .expect("Failed to write object file");

    let gcc_status = process::Command::new("gcc")
        .args(["-static", obj_path, "-o", &args.output])
        .status()
        .expect("Failed to execute gcc");
    if !gcc_status.success() {
        eprintln!("gcc failed");
        std::process::exit(1);
    }
}
