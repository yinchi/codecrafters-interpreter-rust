//! Module for running a Lox program.

use std::collections::HashMap;
use std::rc::Rc;

use crate::builtins::builtins;
use crate::environment::{EnvRc, new_env_rc};
use crate::evaluator::RuntimeError;
use crate::evaluator::{RunError, evaluate, is_truthy};
use crate::parser::{
    ASTree, Declaration, Expression, ExpressionEnum, FunDecl, Literal, LoxClass, Primary,
    PrimaryEnum, Span, Statement, UserCallable,
};
use crate::resolver::{LocalsType, resolve};

/// Program state, containing the current environment.  This is passed around and updated as we
/// run the program.  A new ProgramState is initialized with a global environment containing all
/// built-in functions, and then each declaration is run in sequence, updating the environment as
/// needed (e.g. to add new variable bindings or class definitions, or swapping environments
/// to enter a new scope).
pub struct ProgramState {
    /// The current environment (lexically scoped, Rc-shared so closures can capture it).
    pub env: EnvRc,
}

impl ProgramState {
    pub fn new() -> Self {
        let env: EnvRc = new_env_rc(None);
        // Seed the global environment with all native callables.
        for native in builtins() {
            env.borrow_mut()
                .vars
                .insert(native.name.to_string(), Literal::NativeCallable(native));
        }
        ProgramState { env }
    }
}

/// Runs a Lox program represented as an AST.
pub fn run_program(ast: &ASTree) -> Result<Literal, RunError> {
    match ast {
        ASTree::Program(program) => {
            let locals_map = match resolve(program) {
                Ok(locals) => locals,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(super::EXIT_CODE_SYNTAX_ERROR);
                }
            };
            let mut state: ProgramState = ProgramState::new();
            let mut last_val: Literal = Literal::Nil;
            for decl in &program.declarations {
                last_val = run_decl(decl, &mut state, &locals_map)?;
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

/// Helper function to create a Literal::UserCallable from a FunDecl and its closure environment.
fn make_callable(decl: &FunDecl, env: &EnvRc) -> Literal {
    Literal::UserCallable(UserCallable {
        decl: Rc::new(decl.clone()),
        closure: Rc::clone(env),
        is_initializer: false,
    })
}

/// Helper function to evaluate the superclass expression in a class declaration, returning it as an
/// `Rc<LoxClass>` if it exists and is valid, or None if no superclass is specified. Errors if the
/// superclass expression is present but does not evaluate to a class.
fn get_superclass(
    class_decl: &crate::parser::ClassDecl,
    state: &ProgramState,
    locals_map: &LocalsType,
) -> Result<Option<Rc<LoxClass>>, RunError> {
    // If no superclass is specified, return None. Else, extract the superclass name and token ID.
    let Some((superclass_name, superclass_id)) = &class_decl.superclass else {
        return Ok(None);
    };

    // Create an expression for the superclass name.
    let superclass_expr = Expression::new(
        ExpressionEnum::Primary(Primary::new(
            PrimaryEnum::Identifier(superclass_name.clone(), *superclass_id),
            class_decl.span.clone(),
        )),
        class_decl.span.clone(),
    );

    // Evaluate the superclass expression and check that it evaluates to a class.
    match evaluate(&superclass_expr, &state.env, locals_map)? {
        Literal::Class(superclass) => Ok(Some(superclass)),
        _ => Err(RunError::RuntimeError(RuntimeError {
            message: format!("Superclass '{}' must be a class.", superclass_name),
            span: Span::new(class_decl.span.start.clone(), class_decl.span.end.clone()),
        })),
    }
}

/// Runs a single declaration, updating the program state as needed.
pub fn run_decl(
    decl: &Declaration,
    state: &mut ProgramState,
    locals_map: &LocalsType,
) -> Result<Literal, RunError> {
    match decl {
        Declaration::ClassDecl(class_decl) => {
            let superclass = get_superclass(class_decl, state, locals_map)?;

            // If we have a superclass, create a new environment that has the current environment
            // as its parent and binds `super` to the superclass, so that methods can resolve
            // `super`.
            let methods_env = if let Some(superclass) = &superclass {
                let super_env = new_env_rc(Some(&state.env));
                super_env
                    .borrow_mut()
                    .vars
                    .insert("super".to_string(), Literal::Class(Rc::clone(superclass)));
                super_env
            } else {
                Rc::clone(&state.env)
            };

            // Clone the class declaration's methods into UserCallables that capture the
            // current environment as their closure.
            let methods: HashMap<String, UserCallable> = class_decl
                .methods
                .iter()
                .map(|method_decl| {
                    (
                        method_decl.name.clone(),
                        UserCallable {
                            decl: Rc::new(method_decl.clone()),
                            closure: Rc::clone(&methods_env),
                            // A class method is an initializer if its name is "init".
                            is_initializer: method_decl.name == "init",
                        },
                    )
                })
                .collect();

            // Create the LoxClass and insert it into the current environment before returning it.
            let class = Literal::Class(Rc::new(LoxClass::new(
                class_decl.name.clone(),
                superclass,
                methods,
            )));
            state
                .env
                .borrow_mut()
                .vars
                .insert(class_decl.name.clone(), class.clone());
            Ok(class)
        }
        Declaration::VarDecl(var_decl) => {
            // Handle variable declaration, defaulting to nil if no initializer present.
            let val: Literal = if let Some(initializer) = &var_decl.initializer {
                evaluate(initializer, &state.env, locals_map)?
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
        Declaration::FunDecl(fun_decl) => {
            // Capture the current environment as the closure for lexical scoping.
            let closure: EnvRc = Rc::clone(&state.env);
            let callable: Literal = make_callable(fun_decl, &closure);
            state
                .env
                .borrow_mut()
                .vars
                .insert(fun_decl.name.clone(), callable.clone());
            Ok(callable)
        }
        Declaration::Statement(stmt) => run_stmt(stmt, state, locals_map),
    }
}

pub fn run_stmt(
    stmt: &Statement,
    state: &mut ProgramState,
    locals_map: &LocalsType,
) -> Result<Literal, RunError> {
    match stmt {
        Statement::ExprStmt(expr_stmt) => {
            let ret: Literal = evaluate(&expr_stmt.expr, &state.env, locals_map)?;
            Ok(ret)
        }
        Statement::PrintStmt(print_stmt) => {
            let val: Literal = evaluate(&print_stmt.expr_stmt.expr, &state.env, locals_map)?;
            println!("{}", val);
            // The value of a print statement is always nil.
            // (Check if `return print val;` works in the Lox spec to confirm this.)
            Ok(Literal::Nil)
        }
        Statement::ReturnStmt(return_stmt) => {
            let val: Literal = if let Some(expr) = &return_stmt.value {
                // `return val;` case
                evaluate(expr, &state.env, locals_map)?
            } else {
                // `return;` case, which returns nil.
                Literal::Nil
            };
            Err(RunError::ReturnSignal(val))
        }
        Statement::IfStmt(if_stmt) => {
            let condition: Literal = evaluate(&if_stmt.condition, &state.env, locals_map)?;
            if is_truthy(&condition) {
                // If the condition is truthy, run the then branch and return its value.
                run_stmt(&if_stmt.then_branch, state, locals_map)
            } else if let Some(else_branch) = &if_stmt.else_branch {
                // Else if the condition is falsy and we have an else branch,
                // run the else branch and return its value.
                run_stmt(else_branch, state, locals_map)
            } else {
                // Else if the condition is falsy and we don't have an else branch,
                // so no branch gets executed and the value of the if statement is nil.
                Ok(Literal::Nil)
            }
        }
        Statement::WhileStmt(while_stmt) => {
            while is_truthy(&evaluate(&while_stmt.condition, &state.env, locals_map)?) {
                run_stmt(&while_stmt.body, state, locals_map)?;
            }
            Ok(Literal::Nil)
        }
        Statement::Block(decls) => run_block(decls, state, locals_map),
    }
}

fn run_block(
    decls: &[Declaration],
    state: &mut ProgramState,
    locals_map: &LocalsType,
) -> Result<Literal, RunError> {
    // Create a new child environment with the current env as its parent.
    let child_env: EnvRc = new_env_rc(Some(&state.env));
    let saved_env: EnvRc = std::mem::replace(&mut state.env, child_env);

    // Run each declaration in the block, stopping early on error or return signal.
    let mut result: Result<Literal, RunError> = Ok(Literal::Nil);
    for decl in decls {
        match run_decl(decl, state, locals_map) {
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
