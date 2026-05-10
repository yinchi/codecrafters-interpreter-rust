//! Built-in function definitions.
//!
//! For the Lox language, we only have one built-in function, `clock`, which returns the number of
//! seconds since the Unix epoch as a floating-point number.  This module defines the
//! implementation of `clock` and a `builtins` function that returns a list of all built-in
//! callables to be seeded into the global environment at startup.

use crate::parser::{Literal, NativeCallable};

/// Returns the number of seconds since the Unix epoch as a floating-point number.
fn clock(args: &[Literal]) -> Result<Literal, String> {
    if !args.is_empty() {
        return Err("Runtime error: 'clock' does not take any arguments".into());
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let start: SystemTime = SystemTime::now();
    match start.duration_since(UNIX_EPOCH) {
        Ok(n) => Ok(Literal::Number(n.as_secs_f64())),
        Err(e) => Ok(Literal::Number(-e.duration().as_secs_f64())),
    }
}

/// Returns all built-in callables to be seeded into the global environment.
pub fn builtins() -> Vec<NativeCallable> {
    vec![NativeCallable {
        name: "clock",
        arity: 0,
        func: clock,
    }]
}
