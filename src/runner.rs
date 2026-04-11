//! Module for running a Lox program.

use std::rc::Rc;

use crate::builtins::BuiltIns;
use crate::environment::{EnvRc, new_env_rc};
use crate::evaluator::{RunError, evaluate, is_truthy};
use crate::parser::{ASTree, Callable, Declaration, Literal, Statement};

/// Program state
pub struct ProgramState {
    /// The current environment (lexically scoped, Rc-shared so closures can capture it).
    pub env: EnvRc,
    /// Built-in functions and constants.
    pub builtins: Rc<BuiltIns>,
}

impl ProgramState {
    pub fn new() -> Self {
        ProgramState {
            env: new_env_rc(None),
            builtins: Rc::new(BuiltIns::new()),
        }
    }
}

/// Runs a Lox program represented as an AST.
pub fn run_program(ast: &ASTree) -> Result<Literal, RunError> {
    match ast {
        ASTree::Program(program) => {
            let mut state = ProgramState::new();
            let mut last_val = Literal::Nil;
            for decl in &program.declarations {
                last_val = run_decl(decl, &mut state)?;
            }
            Ok(last_val)
        }
        ASTree::Declaration(_) => {
            // A program is any number of declarations followed by EOF, so if we see a single
            // declaration without EOF, something is wrong (since we should always pick up an
            // EOF token from the input file).
            eprintln!("Syntax error: expected EOF after declaration.");
            std::process::exit(super::EXIT_CODE_SYNTAX_ERROR);
        }
        ASTree::Expr(_) => {
            eprintln!("Syntax error: expected ; but found EOF.");
            std::process::exit(super::EXIT_CODE_SYNTAX_ERROR);
        }
    }
}

/// Runs a single declaration, updating the program state as needed.
fn run_decl(decl: &Declaration, state: &mut ProgramState) -> Result<Literal, RunError> {
    match decl {
        Declaration::VarDecl(var_decl, _) => {
            // Handle variable declaration, defaulting to nil if no initializer present.
            let val = if let Some(initializer) = &var_decl.initializer {
                evaluate(initializer, &state.env, &state.builtins)?
            } else {
                Literal::Nil
            };
            state
                .env
                .borrow_mut()
                .vars
                .insert(var_decl.name.clone(), val.clone());
            // Return the value of the variable declaration, which is the value
            // of the initializer if present, or nil otherwise.
            Ok(val)
        }
        Declaration::FunDecl(fun_decl, _) => {
            // Capture the current environment as the closure for lexical scoping.
            let closure = Rc::clone(&state.env);
            let callable = Literal::Callable(Callable::new(fun_decl.clone(), closure));
            state
                .env
                .borrow_mut()
                .vars
                .insert(fun_decl.name.clone(), callable.clone());
            Ok(callable)
        }
        Declaration::Statement(stmt, _) => run_stmt(stmt, state),
    }
}

pub fn run_stmt(stmt: &Statement, state: &mut ProgramState) -> Result<Literal, RunError> {
    match stmt {
        Statement::ExprStmt(expr_stmt) => {
            let ret = evaluate(&expr_stmt.expr, &state.env, &state.builtins)?;
            Ok(ret)
        }
        Statement::PrintStmt(print_stmt) => {
            let val = evaluate(&print_stmt.expr_stmt.expr, &state.env, &state.builtins)?;
            println!("{}", val);
            // The value of a print statement is always nil.
            // (Check if `return print val;` works in the Lox spec to confirm this.)
            Ok(Literal::Nil)
        }
        Statement::ReturnStmt(return_stmt) => {
            let val = if let Some(expr) = &return_stmt.value {
                // `return val;` case
                evaluate(expr, &state.env, &state.builtins)?
            } else {
                // `return;` case, which returns nil.
                Literal::Nil
            };
            Err(RunError::ReturnSignal(val))
        }
        Statement::IfStmt(if_stmt) => {
            let condition = evaluate(&if_stmt.condition, &state.env, &state.builtins)?;
            if is_truthy(&condition) {
                // If the condition is truthy, run the then branch and return its value.
                run_stmt(&if_stmt.then_branch, state)
            } else if let Some(else_branch) = &if_stmt.else_branch {
                // Else if the condition is falsy and we have an else branch,
                // run the else branch and return its value.
                run_stmt(else_branch, state)
            } else {
                // Else if the condition is falsy and we don't have an else branch,
                // so no branch gets executed and the value of the if statement is nil.
                Ok(Literal::Nil)
            }
        }
        Statement::WhileStmt(while_stmt) => {
            while is_truthy(&evaluate(
                &while_stmt.condition,
                &state.env,
                &state.builtins,
            )?) {
                run_stmt(&while_stmt.body, state)?;
            }
            Ok(Literal::Nil)
        }
        Statement::Block(decls) => run_block(decls, state),
    }
}

fn run_block(decls: &[Declaration], state: &mut ProgramState) -> Result<Literal, RunError> {
    // Create a new child environment with the current env as its parent.
    let child_env = new_env_rc(Some(&state.env));
    let saved_env = std::mem::replace(&mut state.env, child_env);

    // Run each declaration in the block, stopping early on error or return signal.
    let mut result: Result<Literal, RunError> = Ok(Literal::Nil);
    for decl in decls {
        match run_decl(decl, state) {
            Ok(val) => {
                result = Ok(val);
            }
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }

    // Restore the parent environment before returning (even on error/signal).
    state.env = saved_env;

    result
}
