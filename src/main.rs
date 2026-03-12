mod evaluator;
mod format;
mod parser;
mod runner;
mod tokenizer;

use std::env;
use std::fs;

use evaluator::{evaluate, handle_runtime_error};
use parser::parse;
use runner::{Environment, run_program};
use tokenizer::tokenize;

use crate::parser::ASTree;

const EXIT_CODE_SYNTAX_ERROR: i32 = 65;
const EXIT_CODE_RUNTIME_ERROR: i32 = 70;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ./rlox [tokenize | parse | evaluate | run] <filename>");
        return;
    }

    let command = &args[1];
    let filename = &args[2];

    match command.as_str() {
        "tokenize" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                eprintln!("Failed to read file {}", filename);
                String::new()
            });

            let (tokens, has_error) = tokenize(&file_contents);
            for token in tokens {
                println!("{} {} {}", token.token_type, token.lexeme, token.literal);
            }
            if has_error {
                // Tokenizer already printed the error message(s), so just exit with code 65.
                std::process::exit(EXIT_CODE_SYNTAX_ERROR);
            }
        }
        "parse" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                eprintln!("Failed to read file {}", filename);
                String::new()
            });

            let (tokens, has_error) = tokenize(&file_contents);
            if has_error {
                // Tokenizer already printed the error message(s), so just exit with code 65.
                std::process::exit(EXIT_CODE_SYNTAX_ERROR);
            }

            match parse(&tokens) {
                Ok(ast) => {
                    println!("{:?}", ast);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(EXIT_CODE_SYNTAX_ERROR);
                }
            }
        }
        "evaluate" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                eprintln!("Failed to read file {}", filename);
                String::new()
            });

            let (tokens, has_error) = tokenize(&file_contents);
            if has_error {
                // Tokenizer already printed the error message(s), so just exit with code 65.
                std::process::exit(EXIT_CODE_SYNTAX_ERROR);
            }

            match parse(&tokens) {
                Ok(ast) => {
                    // If the AST corresponds to an expression, evaluate it and print the result.
                    // Else raise an error.  In evaluation mode, we don't have any preceding
                    // variable declarations, so use an empty environment for evaluation.
                    if let ASTree::Expr(expr) = ast {
                        let mut env = Environment::new();
                        match evaluate(&expr, &mut env) {
                            Ok(value) => println!("{}", value),
                            Err(e) => {
                                handle_runtime_error(&e, file_contents.as_str());
                                std::process::exit(EXIT_CODE_RUNTIME_ERROR);
                            }
                        }
                    } else {
                        eprintln!("Only expression statements can be evaluated.");
                        std::process::exit(EXIT_CODE_SYNTAX_ERROR);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(EXIT_CODE_SYNTAX_ERROR);
                }
            }
        }
        "run" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                eprintln!("Failed to read file {}", filename);
                String::new()
            });

            let (tokens, has_error) = tokenize(&file_contents);
            if has_error {
                // Tokenizer already printed the error message(s), so just exit with code 65.
                std::process::exit(EXIT_CODE_SYNTAX_ERROR);
            }

            match parse(&tokens) {
                Ok(ast) => {
                    if let Err(e) = run_program(&ast) {
                        handle_runtime_error(&e, file_contents.as_str());
                        std::process::exit(EXIT_CODE_RUNTIME_ERROR);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(EXIT_CODE_SYNTAX_ERROR);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
        }
    }
}
