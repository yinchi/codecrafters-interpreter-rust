//! Environment (scope) for Lox variable bindings.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::parser::ast::Literal;

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

/// Create a new EnvRc, optionally chained to a parent environment.
pub fn new_env_rc(parent: Option<&EnvRc>) -> EnvRc {
    Rc::new(RefCell::new(Environment {
        vars: HashMap::new(),
        parent: parent.map(Rc::clone),
    }))
}
