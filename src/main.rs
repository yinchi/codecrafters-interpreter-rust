mod builtins;
mod environment;
mod evaluator;
mod format;
mod parser;
mod resolver;
mod runner;
mod tokenizer;

use std::env;
use std::fs;

use evaluator::{RunError, evaluate, handle_runtime_error};
use parser::parse;
use runner::{ProgramState, run_program};
use tokenizer::tokenize;

use crate::parser::ASTree;
use crate::resolver::LocalsType;

const EXIT_CODE_SYNTAX_ERROR: i32 = 65;
const EXIT_CODE_RUNTIME_ERROR: i32 = 70;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ./rlox [tokenize | parse | evaluate | run] <filename>");
        return;
    }

    let command: &String = &args[1];
    let filename: &String = &args[2];

    match command.as_str() {
        "tokenize" => {
            let file_contents: String = fs::read_to_string(filename).unwrap_or_else(|_| {
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
            let file_contents: String = fs::read_to_string(filename).unwrap_or_else(|_| {
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
                        let state = ProgramState::new();
                        match evaluate(&expr, &state.env, &LocalsType::new()) {
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
            let file_contents: String = fs::read_to_string(filename).unwrap_or_else(|_| {
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
                    match run_program(&ast) {
                        Ok(_) | Err(RunError::ReturnSignal(_)) => {
                            // If the program returns a value, ignore it and exit with code 0.
                        }
                        Err(RunError::RuntimeError(e)) => {
                            handle_runtime_error(&e, file_contents.as_str());
                            std::process::exit(EXIT_CODE_RUNTIME_ERROR);
                        }
                    }
                    // If Ok or ReturnSignal, the program has finished normally.
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
