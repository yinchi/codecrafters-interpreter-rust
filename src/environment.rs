//! Environment (scope) for Lox variable bindings.
//!
//! The `Environment` struct represents a single scope of variable bindings, and the `EnvRc` type
//! is a reference-counted pointer to an `Environment`, allowing multiple closures to share the
//! same environment.  The `new_env_rc` function is a helper for creating a new `EnvRc`, optionally
//! chained to a parent environment.
//!
//! The `Environment` struct has two main fields:
//!
//! - `vars`, a `HashMap` that stores variable names and their corresponding `Literal` values.
//! - `parent`, an optional `EnvRc` that points to the parent environment, enabling lexical scoping.
//!
//! The `Environment` struct also has two key methods:
//!
//! - `get_at`, which looks up a variable at a specific distance up the environment chain and
//!   returns its value if found.
//! - `assign_at`, which assigns a new value to a variable at a specific distance up the
//!   environment chain, returning `true` if the variable was found and updated, `false` if the
//!   name is absent at that scope.
//!
//! The appropiate distance for variable lookups and assignments is determined by the *resolver*,
//! which computes the distance for each variable during the static analysis phase.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::parser::Literal;

/// An `Rc<RefCell<Environment>>` is used to represent the current environment, which is shared
/// between the main program and any closures that capture it. The `Rc` allows multiple closures to
/// share the same environment, and the `RefCell` allows mutation of the environment (eg. variable
/// assignment) even when it's shared.
pub type EnvRc = Rc<RefCell<Environment>>;

/// Stores variable bindings for a scope.  The parent chain implements lexical scoping.
pub struct Environment {
    pub vars: HashMap<String, Literal>,
    pub parent: Option<EnvRc>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
            parent: None,
        }
    }

    /// Look up a variable exactly `distance` hops up the environment chain.
    /// Panics if the chain is shorter than `distance` (resolver bug).
    pub fn get_at(env: &EnvRc, distance: usize, name: &str) -> Option<Literal> {
        let mut current = Rc::clone(env);
        for _ in 0..distance {
            let parent = current
                .borrow()
                .parent
                .as_ref()
                .expect("resolver produced invalid depth")
                .clone();
            current = parent;
        }
        current.borrow().vars.get(name).cloned()
    }

    /// Assign to a variable exactly `distance` hops up the environment chain.
    /// Returns `true` if found and updated, `false` if the name is absent at that scope
    /// (resolver bug — the variable should always be present at the resolved depth).
    pub fn assign_at(env: &EnvRc, distance: usize, name: &str, val: Literal) -> bool {
        let mut current = Rc::clone(env);
        for _ in 0..distance {
            let parent = current
                .borrow()
                .parent
                .as_ref()
                .expect("resolver produced invalid depth")
                .clone();
            current = parent;
        }
        if current.borrow().vars.contains_key(name) {
            current.borrow_mut().vars.insert(name.to_string(), val);
            true
        } else {
            false
        }
    }
}

/// Create a new EnvRc, optionally chained to a parent environment.
pub fn new_env_rc(parent: Option<&EnvRc>) -> EnvRc {
    Rc::new(RefCell::new(Environment {
        vars: HashMap::new(),
        parent: parent.map(Rc::clone),
    }))
}
