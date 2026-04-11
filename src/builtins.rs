//! Built-in function definitions.

use std::collections::HashMap;

use crate::parser::Literal;

pub struct FnCallError {
    pub message: String,
}

fn clock(args: &[Literal]) -> Result<Literal, FnCallError> {
    // Ensure that `clock` is called with no arguments, since that's how it's defined in Lox.
    if !args.is_empty() {
        return Err(FnCallError {
            message: "Runtime error: 'clock' does not take any arguments".into(),
        });
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH);
    match since_the_epoch {
        Ok(n) => Ok(Literal::Number(n.as_secs_f64())),
        Err(_) => {
            let before_the_epoch = start.duration_since(UNIX_EPOCH);
            match before_the_epoch {
                Ok(n) => Ok(Literal::Number(-n.as_secs_f64())),
                Err(_) => {
                    // This should never happen, but if it does, just return 0.0.
                    Ok(Literal::Number(0.0))
                }
            }
        }
    }
}

type Function = fn(&[Literal]) -> Result<Literal, FnCallError>;

#[allow(dead_code)]
pub struct BuiltIns {
    pub functions: HashMap<String, Function>,
}

impl BuiltIns {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut functions: HashMap<String, Function> = HashMap::new();
        functions.insert("clock".to_string(), clock);
        BuiltIns { functions }
    }
}
