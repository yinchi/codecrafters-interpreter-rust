//! Module for running a Lox program.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::evaluator::{RuntimeError, evaluate, is_truthy};
use crate::parser::{ASTree, Declaration, Literal, Statement};

/// Program state
struct ProgramState {
    /// The global environment for the program.
    env: Environment,
}

pub struct Environment {
    /// A mapping from variable names to their values (Lox literals).
    pub vars: HashMap<String, Literal>,
    /// The parent environment, if any (for nested scopes).
    pub parent: Option<Rc<RefCell<Environment>>>,
}

impl ProgramState {
    pub fn new() -> Self {
        ProgramState {
            env: Environment::new(),
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
            parent: None,
        }
    }

    /// Look up a variable by walking the environment chain.
    pub fn get_var(&self, name: &str) -> Option<Literal> {
        if let Some(val) = self.vars.get(name) {
            return Some(val.clone());
        }
        self.parent.as_ref()?.borrow().get_var(name)
    }

    /// Assign to an existing variable somewhere in the environment chain.
    /// Returns `true` if the variable was found and updated, `false` if undeclared.
    pub fn set_var(&mut self, name: &str, val: Literal) -> bool {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), val);
            return true;
        }
        if let Some(parent) = &self.parent {
            return parent.borrow_mut().set_var(name, val);
        }
        false
    }
}

/// Runs a Lox program represented as an AST.
pub fn run_program(ast: &ASTree) -> Result<(), RuntimeError> {
    match ast {
        ASTree::Program(program) => {
            let mut state = ProgramState::new();
            for decl in &program.declarations {
                run_decl(decl, &mut state)?;
            }
            Ok(())
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
fn run_decl(decl: &Declaration, state: &mut ProgramState) -> Result<(), RuntimeError> {
    match decl {
        Declaration::VarDecl(var_decl, _) => {
            // Handle variable declaration, defaulting to nil if no initializer present.
            // We don't need to do anything special with the indent depth (_),
            // as `state.env` will already be the correct environment for this statement.
            let val = if let Some(initializer) = &var_decl.initializer {
                evaluate(initializer, &mut state.env)?
            } else {
                Literal::Nil
            };
            state.env.vars.insert(var_decl.name.clone(), val);
        }
        Declaration::Statement(stmt, _) => {
            // We don't need to do anything special with the indent depth (_),
            // as `state.env` will already be the correct environment for this statement.
            run_stmt(stmt, state)?;
        }
    }
    Ok(())
}

fn run_stmt(stmt: &Statement, state: &mut ProgramState) -> Result<(), RuntimeError> {
    match stmt {
        Statement::ExprStmt(expr_stmt) => {
            // Evaluate the expression and discard the result.
            // Ensures side effects are applied, even if the result is not used.
            let _ = evaluate(&expr_stmt.expr, &mut state.env)?;
        }
        Statement::PrintStmt(print_stmt) => {
            let val = evaluate(&print_stmt.expr_stmt.expr, &mut state.env)?;
            println!("{}", val);
        }
        Statement::IfStmt(if_stmt) => {
            let condition = evaluate(&if_stmt.condition, &mut state.env)?;
            if is_truthy(&condition) {
                run_stmt(&if_stmt.then_branch, state)?;
            } else if let Some(else_branch) = &if_stmt.else_branch {
                run_stmt(else_branch, state)?;
            }
        }
        Statement::WhileStmt(while_stmt) => {
            while is_truthy(&evaluate(&while_stmt.condition, &mut state.env)?) {
                run_stmt(&while_stmt.body, state)?;
            }
        }
        Statement::Block(decls) => {
            // Swap out the current env and make it the parent of a new child env.
            // The child env starts empty, and will be populated with any declarations in the block.
            // `get_var` and `set_var` will automatically walk up to the parent env if a variable
            // is not found in the child env.
            let parent_env = std::mem::replace(&mut state.env, Environment::new());
            state.env.parent = Some(Rc::new(RefCell::new(parent_env)));

            // Run each declaration in the block using the child environment.
            for decl in decls {
                run_decl(decl, state)?;
            }

            // Restore the parent environment when the block exits.
            let parent_rc = state.env.parent.take().unwrap();
            state.env = Rc::try_unwrap(parent_rc).ok().unwrap().into_inner();
        }
    }
    Ok(())
}
