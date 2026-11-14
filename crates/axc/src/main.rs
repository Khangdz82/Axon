//! Axon Compiler CLI Driver (`axc`)

use axon_ast::AstArena;
use axon_lexer::Lexer;
use axon_parser::Parser;
use axon_span::SourceMap;
use bumpalo::Bump;
use lasso::Rodeo;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: axc <check|build|run> <file.ax>");
        process::exit(1);
    }

    let cmd = &args[1];
    if cmd == "--version" || cmd == "-v" {
        println!("axc 0.1.0 (Axon Systems Language Compiler)");
        return;
    }

    if args.len() < 3 {
        eprintln!("Error: missing input file");
        process::exit(1);
    }

    let file_path = &args[2];
    let src = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    let mut sm = SourceMap::new();
    let file_id = sm.add_file(file_path.clone(), src.clone());

    let mut interner = Rodeo::default();
    let lexer = Lexer::new(&src, file_id, &mut interner);
    let tokens = match lexer.tokenize_all() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", sm.format_error(e.span, &e.msg));
            process::exit(1);
        }
    };

    let bump = Bump::new();
    let mut arena = AstArena::new(&bump);
    let mut parser = Parser::new(&tokens, &mut arena, &mut interner);

    match parser.parse_program() {
        Ok(prog) => {
            println!(
                "[+] Successfully parsed '{}': {} struct(s), {} function(s)",
                file_path,
                prog.structs.len(),
                prog.functions.len()
            );
        }
        Err(e) => {
            eprintln!("{}", sm.format_error(e.span, &e.msg));
            process::exit(1);
        }
    }
}
