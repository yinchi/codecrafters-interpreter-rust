//! Module for evaluating expressions in the AST.
//!
//! This module defines the `evaluate` function, which takes an expression from the AST and
//! computes its value according to Lox's semantics. This function is called by the main program
//! when the user runs in "evaluate" mode and a single expression is found by the parser
//! (else an error is raised). Evaluation is also used in "run" mode when executing statements that
//! contain expressions.
//!
//! The `evaluate` function recursively evaluates sub-expressions, reducing them to their literal
//! values, until the entire expression is reduced to a single `Literal` or a runtime error occurs.

use std::cell::RefCell;
use std::rc::Rc;

use super::EXIT_CODE_RUNTIME_ERROR;
use crate::environment::EnvRc;
use crate::environment::Environment;
use crate::parser::Location;
use crate::parser::ast::{
    Expression, ExpressionEnum, Literal, Operator, Primary, PrimaryEnum, Statement,
};
use crate::parser::span::Span;
use crate::resolver::LocalsType;
use crate::runner::ProgramState;

/// An error that occurs during evaluation of an expression, e.g. division by zero or
/// operations on incompatible types.
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

/// The error type for the runner: either a runtime error or a return signal.
pub enum RunError {
    RuntimeError(RuntimeError),
    ReturnSignal(Literal),
}

impl From<RuntimeError> for RunError {
    fn from(e: RuntimeError) -> Self {
        RunError::RuntimeError(e)
    }
}

/// Evaluates an expression in the AST and returns its value as a string.
pub fn evaluate(
    expr: &Expression,
    env: &EnvRc,
    locals_map: &LocalsType,
) -> Result<Literal, RuntimeError> {
    match &expr.expr {
        ExpressionEnum::Primary(p) => eval_primary(p, env, locals_map),

        ExpressionEnum::Unary(op, right) => eval_unary(op, right, &expr.span, env, locals_map),

        ExpressionEnum::Factor(op, left, right)
        | ExpressionEnum::Term(op, left, right)
        | ExpressionEnum::Comparison(op, left, right)
        | ExpressionEnum::Equality(op, left, right) => {
            eval_infix(op, left, right, &expr.span, env, locals_map)
        }

        ExpressionEnum::Logical(op, left, right) => eval_logical(op, left, right, env, locals_map),

        ExpressionEnum::Assignment(name, val) => {
            if let PrimaryEnum::Identifier(var_name, id) = &name.p {
                let val = evaluate(val, env, locals_map)?;
                let found = if let Some(&depth) = locals_map.get(id) {
                    Environment::assign_at(env, depth, var_name, val.clone())
                } else {
                    // Not in locals_map — evaluate mode with no resolver pass.
                    env.borrow_mut()
                        .vars
                        .contains_key(var_name)
                        .then(|| {
                            env.borrow_mut()
                                .vars
                                .insert(var_name.to_string(), val.clone());
                        })
                        .is_some()
                };
                if found {
                    Ok(val)
                } else {
                    Err(RuntimeError {
                        message: format!("Runtime error: undefined variable '{}'", var_name),
                        span: name.span.clone(),
                    })
                }
            } else {
                unreachable!() // parser only creates Assignment with an Identifier LHS
            }
        }

        ExpressionEnum::Call(callee, args) => {
            // Evaluate the callee to get a value, then dispatch on its type.
            // This naturally handles simple identifiers, chained calls like `f()()`, etc.
            // NativeCallable and UserCallable are both ordinary Literal values in the env.
            let callee_val: Literal = evaluate(callee, env, locals_map)?;
            match callee_val {
                Literal::NativeCallable(native) => {
                    if args.args.len() != native.arity {
                        return Err(RuntimeError {
                            message: format!(
                                "Runtime error: expected {} arguments but got {}",
                                native.arity,
                                args.args.len()
                            ),
                            span: expr.span.clone(),
                        });
                    }
                    let arg_vals: Vec<Literal> = args
                        .args
                        .iter()
                        .map(|arg| evaluate(arg, env, locals_map))
                        .collect::<Result<Vec<Literal>, RuntimeError>>()?;
                    (native.func)(&arg_vals).map_err(|msg| RuntimeError {
                        message: msg,
                        span: args.span.clone(),
                    })
                }
                Literal::UserCallable(callable) => {
                    let arity: usize = callable.decl.params.len();
                    if args.args.len() != arity {
                        return Err(RuntimeError {
                            message: format!(
                                "Runtime error: expected {} arguments but got {}",
                                arity,
                                args.args.len()
                            ),
                            span: expr.span.clone(),
                        });
                    }

                    // Evaluate arguments before touching the environment.
                    let arg_vals: Vec<Literal> = args
                        .args
                        .iter()
                        .map(|arg| evaluate(arg, env, locals_map))
                        .collect::<Result<Vec<Literal>, RuntimeError>>()?;

                    // Build the function's own scope with the closure as parent (lexical scoping).
                    let mut fn_env: Environment = Environment::new();
                    fn_env.parent = Some(Rc::clone(&callable.closure));

                    // Bind the function's parameters to the argument values in the function scope.
                    for ((param, _loc), val) in callable.decl.params.iter().zip(arg_vals) {
                        fn_env.vars.insert(param.clone(), val);
                    }

                    let mut state: ProgramState = ProgramState {
                        env: Rc::new(RefCell::new(fn_env)),
                    };

                    // Run the body block's declarations directly in fn_env (no child scope),
                    // matching the resolver which merges the parameter scope and body block
                    // scope into one.  This lets redeclaration of params be caught by the
                    // resolver, and keeps the environment depth in sync with resolver depths.
                    let body_decls = match callable.decl.body.as_ref() {
                        Statement::Block(decls) => decls,
                        _ => unreachable!("function body must be a block"),
                    };

                    // Evaluate the function body, catching ReturnSignal as a normal return value.
                    match (|| {
                        let mut last = Literal::Nil;
                        for decl in body_decls {
                            last = crate::runner::run_decl(decl, &mut state, locals_map)?;
                        }
                        Ok(last)
                    })() {
                        Ok(val) => Ok(val),
                        Err(RunError::ReturnSignal(val)) => Ok(val),
                        Err(RunError::RuntimeError(e)) => Err(e),
                    }
                }
                _ => Err(RuntimeError {
                    message: "Runtime error: calling a non-callable value".into(),
                    span: callee.span.clone(),
                }),
            }
        }
    }
}

/// Evaluates a primary expression, which can be a literal, grouping, or identifier.
fn eval_primary(
    primary: &Primary,
    env: &EnvRc,
    locals_map: &LocalsType,
) -> Result<Literal, RuntimeError> {
    match &primary.p {
        PrimaryEnum::Literal(lit) => Ok(lit.clone()),
        PrimaryEnum::Grouping(expr) => evaluate(expr, env, locals_map),
        PrimaryEnum::Identifier(name, id) => {
            let found = if let Some(&depth) = locals_map.get(id) {
                Environment::get_at(env, depth, name)
            } else {
                // Not in locals_map — evaluate mode with no resolver pass; fall back to
                // current env (which is global in evaluate mode).
                env.borrow().vars.get(name).cloned()
            };
            match found {
                Some(val) => Ok(val),
                None => Err(RuntimeError {
                    message: format!("Runtime error: undefined variable '{}'", name),
                    span: primary.span.clone(),
                }),
            }
        }
    }
}

/// Evaluate a binary infix expression.
///
/// Handles arithmetic (`+`, `-`, `*`, `/`), comparison (`<`, `<=`, `>`, `>=`),
/// and equality (`==`, `!=`) operators.  Comparison and equality operators return
/// `Literal::True` or `Literal::False`.
fn eval_infix(
    op: &Operator,
    left: &Expression,
    right: &Expression,
    span: &Span,
    env: &EnvRc,
    locals_map: &LocalsType,
) -> Result<Literal, RuntimeError> {
    let left_val: Literal = evaluate(left, env, locals_map)?;
    let right_val: Literal = evaluate(right, env, locals_map)?;

    // String concatenation with "+"
    if op.op.as_str() == "+"
        && let (Literal::String(l), Literal::String(r)) = (&left_val, &right_val)
    {
        return Ok(Literal::String(format!("{}{}", l, r)));
    }

    // Equality operators work on any type
    match op.op.as_str() {
        "==" => {
            return Ok(if left_val == right_val {
                Literal::True
            } else {
                Literal::False
            });
        }
        "!=" => {
            return Ok(if left_val != right_val {
                Literal::True
            } else {
                Literal::False
            });
        }
        _ => {} // Fall through to next block for other operators
    }

    // All remaining operators require both operands to be numbers
    if let (Literal::Number(l), Literal::Number(r)) = (&left_val, &right_val) {
        // Comparison operators return boolean results
        match op.op.as_str() {
            "<" => return Ok(if l < r { Literal::True } else { Literal::False }),
            "<=" => {
                return Ok(if l <= r {
                    Literal::True
                } else {
                    Literal::False
                });
            }
            ">" => return Ok(if l > r { Literal::True } else { Literal::False }),
            ">=" => {
                return Ok(if l >= r {
                    Literal::True
                } else {
                    Literal::False
                });
            }
            _ => {} // Fall through to next block for arithmetic operators
        }

        // Arithmetic operators return number results
        let result = match op.op.as_str() {
            "*" => l * r,
            "/" => {
                if *r == 0.0 {
                    return Err(RuntimeError {
                        message: "Runtime error: division by zero".into(),
                        span: right.span.clone(),
                    });
                }
                l / r
            }
            "+" => l + r,
            "-" => l - r,
            _ => unreachable!(), // Factor/Term can only be *, /, +, -
        };
        return Ok(Literal::Number(result));
    }

    Err(RuntimeError {
        message: format!(
            "Runtime error: operator '{:?}' cannot be applied to values '{}' and '{}'",
            op, left_val, right_val
        ),
        span: span.clone(),
    })
}

/// Evaluate an unary expression.
fn eval_unary(
    op: &Operator,
    right: &Expression,
    span: &Span,
    env: &EnvRc,
    locals_map: &LocalsType,
) -> Result<Literal, RuntimeError> {
    let right_val: Literal = evaluate(right, env, locals_map)?;
    match op.op.as_str() {
        "-" => {
            // Try to parse right_val as a number, negate it, and return the result as a string
            match right_val {
                Literal::Number(n) => Ok(Literal::Number(-n)),
                _ => Err(RuntimeError {
                    message: format!(
                        "Runtime error: cannot apply unary '-' to non-number value '{}'",
                        right_val
                    ),
                    span: span.clone(),
                }),
            }
        }
        "!" => {
            // Apply logical NOT
            if is_truthy(&right_val) {
                Ok(Literal::False)
            } else {
                Ok(Literal::True)
            }
        }
        _ => unreachable!(), // parser only creates Unary for '-' and '!'
    }
}

/// Check a value for truthiness.
///
/// In Lox, only "`false`" and "`nil`" are falsy; everything else is truthy.
pub fn is_truthy(value: &Literal) -> bool {
    !matches!(value, Literal::False | Literal::Nil)
}

/// Evaluate a logical expression (AND/OR).  Implements short-circuiting semantics.
fn eval_logical(
    op: &Operator,
    left: &Expression,
    right: &Expression,
    env: &EnvRc,
    locals_map: &LocalsType,
) -> Result<Literal, RuntimeError> {
    match op.op.as_str() {
        "and" => {
            let left_val: Literal = evaluate(left, env, locals_map)?;
            if !is_truthy(&left_val) {
                Ok(left_val) // Short-circuit: return left operand if it's falsy
            } else {
                evaluate(right, env, locals_map) // Otherwise evaluate and return right operand
            }
        }
        "or" => {
            let left_val: Literal = evaluate(left, env, locals_map)?;
            if is_truthy(&left_val) {
                Ok(left_val) // Short-circuit: return left operand if it's truthy
            } else {
                evaluate(right, env, locals_map) // Otherwise evaluate and return right operand
            }
        }
        _ => unreachable!(), // parser only creates Logical for 'and' and 'or'
    }
}

/// Format a runtime error with source code context and return it as a string.
pub fn handle_runtime_error(e: &RuntimeError, source: &str) -> () {
    let start_pos: &Location = &e.span.start;
    let end_pos: &Location = &e.span.end;
    if start_pos.line == end_pos.line {
        // Single-line error
        // Print the line with the error and an underline pointing to the error span
        let line_text = source.lines().nth(start_pos.line - 1).unwrap_or("");
        let underline = " ".repeat(start_pos.col - 1) + &"^".repeat(end_pos.col - start_pos.col);
        let msg_line_fragment = format!("[line {}]: ", start_pos.line);
        eprintln!("{}{}", msg_line_fragment, line_text);
        eprintln!("{}{}", " ".repeat(msg_line_fragment.len()), underline);

        // Display the error message below the line
        eprintln!();
        eprintln!("{}", e.message);
        std::process::exit(EXIT_CODE_RUNTIME_ERROR);
    } else {
        // Multi-line error
        eprintln!("[lines {}-{}]:", start_pos.line, end_pos.line);

        // How many columns are needed to display the line numbers for the affected lines?
        // E.g. log10(9999) = 3.999..., floor to 3 and add 1 to get 4 digits
        let width = (end_pos.line as f64).log10().floor() as usize + 1;

        // Print all affected lines with line numbers
        for (i, line) in source
            .lines()
            .enumerate()
            .skip(start_pos.line - 1)
            .take(end_pos.line - start_pos.line + 1)
        {
            eprintln!("{:>width$} | {}", i + 1, line, width = width);
        }

        // Display the error message below the line
        eprintln!();
        eprintln!("{}", e.message);
        std::process::exit(EXIT_CODE_RUNTIME_ERROR);
    }
}
