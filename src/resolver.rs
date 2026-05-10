//! Module for resolving variable references and scopes.
//!
//! This module defines the `resolve` function, which takes an expression or statement from the AST
//! and determines the scope of each variable. This is used to ensure that variables are properly
//! declared before use and to support lexical scoping.

use std::collections::HashMap;

use crate::parser::{Location, ast};

/// A stack of scopes, where each scope is a mapping of variable names to a boolean indicating
/// whether the variable has been fully initialized (`true`) or only declared (`false`).
type ScopesType = Vec<HashMap<String, bool>>;

/// Mapping of identifier IDs to their corresponding scope depth.
pub type LocalsType = HashMap<usize, usize>;

/// Tracks what kind of function (if any) we are currently resolving inside.
#[derive(Clone, Copy, PartialEq)]
enum FunctionType {
    None,
    Function,
    Method,
    Initializer,
}

#[derive(Clone, Copy, PartialEq)]
enum ClassType {
    None,
    Class,
    Subclass,
}

/** Context for resolving variable references in the AST.  The resolver walks the AST and tracks
 * the current scope stack and variable declarations to resolve variable references and detect
 * resolver errors (e.g. referencing a variable in its own initializer, or using `this` outside of
 * a class).  The resolver produces a mapping of identifier IDs to their scope depth, which is
 * used by the interpreter to look up variable values at runtime.
 */
#[derive(derive_new::new)]
struct Resolver {
    /// Stack of scopes, where each scope is a mapping of variable names to a boolean indicating
    /// whether the variable has been fully initialized (`true`) or only declared (`false`).
    /// An empty stack means we're in the global scope.  Scopes can be pushed and popped using
    /// `begin_scope` and `end_scope`.
    scopes: ScopesType,

    /// Mapping of identifier IDs to their corresponding scope depth. This is used to resolve
    /// variable references.  Since each identifier corresponds to a single token, we can use the
    /// token's index in the `tokenize()` output as the identifier ID.
    locals: LocalsType,

    /// Tracks whether the resolver is currently inside a function body.
    current_function: FunctionType,

    /// Tracks whether the resolver is currently inside a class declaration.
    current_class: ClassType,
}

impl Resolver {
    /// Resolves variable references in the given AST and returns a mapping of identifier IDs to their
    /// corresponding scope depth.
    pub fn resolve_program(&mut self, prog: &ast::Program) -> Result<LocalsType, String> {
        for decl in &prog.declarations {
            self.resolve_declaration(decl)?;
        }
        Ok(self.locals.clone())
    }

    /// Declares a variable in the current scope.  Returns an error if the variable is
    /// already declared in the current scope, however, this does not apply if `self.scopes` is
    /// empty (global scope).
    fn declare(&mut self, name: &str, loc: Option<&Location>) -> Result<(), String> {
        if let Some(scope) = self.scopes.last()
            && scope.contains_key(name)
        {
            return Err(match loc {
                Some(l) => format!(
                    "[line {}:{}] Syntax error: variable '{}' already declared in this scope.",
                    l.line, l.col, name
                ),
                None => format!(
                    "Syntax error: variable '{}' already declared in this scope.",
                    name
                ),
            });
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), false);
        }
        Ok(())
    }

    /// Defines a variable in the current scope.  This should be called after the variable's
    /// initializer expression (if any) has been resolved, to indicate that the variable is fully
    /// initialized and can be referenced.
    fn define(&mut self, name: &str) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), true);
            } else {
                return Err(format!(
                    "Syntax error: variable '{}' not declared in this scope.",
                    name
                ));
            }
        }
        Ok(())
    }

    /// Begins a new scope by pushing a new empty scope onto the stack.
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Ends the current scope by popping the top scope from the stack.
    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    /// Resolves variable references in the given declaration.
    fn resolve_declaration(&mut self, decl: &ast::Declaration) -> Result<(), String> {
        match decl {
            ast::Declaration::ClassDecl(class_decl) => {
                // Enter a Class context to track that we're resolving inside a class declaration
                let enclosing_class = self.current_class;
                self.current_class = ClassType::Class;

                // Declare and define the class name
                self.declare(&class_decl.name, Some(&class_decl.span.start))?;
                self.define(&class_decl.name)?;

                // Resolve the superclass (if any). Reject cases like `class A < A { ... }`
                // where a class tries to inherit from itself.
                if let Some((superclass_name, superclass_loc)) = &class_decl.superclass {
                    if superclass_name == &class_decl.name {
                        return Err(format!(
                            "[line {}:{}] Syntax error: a class can't inherit from itself.",
                            class_decl.span.start.line, class_decl.span.start.col
                        ));
                    }

                    // Change the class context to Subclass
                    self.current_class = ClassType::Subclass;

                    self.resolve_primary(&ast::Primary {
                        p: ast::PrimaryEnum::Identifier(superclass_name.clone(), *superclass_loc),
                        span: class_decl.span.clone(),
                    })?;
                }

                // If in a subclass, create a new scope for `super` and bind it to allow method
                // bodies to resolve `super`.
                if self.current_class == ClassType::Subclass {
                    self.begin_scope();
                    self.declare("super", Some(&class_decl.span.start))?;
                    self.define("super")?;
                }

                // Declare and define each method name in the class scope, then resolve the method
                // bodies in a new scope.
                for method in &class_decl.methods {
                    // Enter a new Function scope for the method body
                    let enclosing = self.current_function;
                    self.current_function = if method.name == "init" {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    };

                    // Outer method scope holds `this`.
                    self.begin_scope();

                    // Bind `this` in method scope so method bodies can resolve it.
                    self.declare("this", Some(&method.span.start))?;
                    self.define("this")?;

                    // Inner scope holds parameters and method locals.
                    self.begin_scope();

                    // Add the method parameters to the method body scope
                    for (param, loc) in &method.params {
                        self.declare(param, Some(loc))?;
                        self.define(param)?;
                    }

                    // Resolve the method body block's declarations in the method body scope.
                    if let ast::Statement::Block(decls) = method.body.as_ref() {
                        for decl in decls {
                            self.resolve_declaration(decl)?;
                        }
                    }

                    // Exit the inner scope
                    self.end_scope();

                    // Exit the outer method scope, which also unbinds `this`, and restore the
                    // enclosing function context.
                    self.end_scope();
                    self.current_function = enclosing;
                }

                // Exit the superclass scope (if any) and restore the enclosing class context.
                if self.current_class == ClassType::Subclass {
                    self.end_scope();
                }

                // Restore the enclosing class context.
                self.current_class = enclosing_class;
                Ok(())
            }
            ast::Declaration::FunDecl(fun_decl) => {
                // Bind the function name in the current scope, then resolve the function body in a new scope.
                self.declare(&fun_decl.name, None)?;
                self.define(&fun_decl.name)?;
                let enclosing = self.current_function;
                self.current_function = FunctionType::Function;
                self.begin_scope();
                for (param, loc) in &fun_decl.params {
                    self.declare(param, Some(loc))?;
                    self.define(param)?;
                }
                // Resolve the body block's declarations directly in the parameter scope,
                // rather than calling resolve_statement on the Block (which would open
                // a redundant child scope, causing param redeclarations to go undetected).
                if let ast::Statement::Block(decls) = fun_decl.body.as_ref() {
                    for decl in decls {
                        self.resolve_declaration(decl)?;
                    }
                } else {
                    self.resolve_statement(&fun_decl.body)?;
                }
                self.end_scope();
                self.current_function = enclosing;
                Ok(())
            }
            ast::Declaration::VarDecl(var_decl) => {
                // Declare the variable first (marking it as not yet fully initialized), then
                // resolve the initializer expression (if any) before defining it.  This ordering
                // ensures that the initializer cannot reference the variable being declared
                // (e.g. `var a = a;` is a resolver error).
                self.declare(&var_decl.name, Some(&var_decl.name_loc))?;
                if let Some(initializer) = &var_decl.initializer {
                    self.resolve_expression(initializer)?;
                }
                self.define(&var_decl.name)?;
                Ok(())
            }
            ast::Declaration::Statement(stmt) => self.resolve_statement(stmt),
        }
    }

    /// Resolves variable references in the given statement.
    fn resolve_statement(&mut self, stmt: &ast::Statement) -> Result<(), String> {
        match stmt {
            ast::Statement::ExprStmt(stmt) => self.resolve_expression(&stmt.expr),
            ast::Statement::PrintStmt(stmt) => self.resolve_expression(&stmt.expr_stmt.expr),
            ast::Statement::ReturnStmt(stmt) => {
                if self.current_function == FunctionType::None {
                    return Err(format!(
                        "[line {}] Error at 'return': Can't return from top-level code.",
                        stmt.span.start.line
                    ));
                }
                if let Some(expr) = &stmt.value {
                    if self.current_function == FunctionType::Initializer {
                        // We can use `return` in an initializer, but only with no value
                        //(i.e. `return;`).
                        return Err(format!(
                            "[line {}] Error at 'return': Can't return a value from an initializer.",
                            stmt.span.start.line
                        ));
                    }
                    self.resolve_expression(expr)?;
                }
                Ok(())
            }
            ast::Statement::IfStmt(stmt) => {
                self.resolve_expression(&stmt.condition)?;
                self.resolve_statement(&stmt.then_branch)?;
                if let Some(else_branch) = &stmt.else_branch {
                    self.resolve_statement(else_branch)?;
                }
                Ok(())
            }
            ast::Statement::WhileStmt(stmt) => {
                self.resolve_expression(&stmt.condition)?;
                self.resolve_statement(&stmt.body)?;
                Ok(())
            }
            ast::Statement::Block(decls) => {
                // Variables in a block are resolved within a new scope.
                self.begin_scope();
                for decl in decls {
                    self.resolve_declaration(decl)?;
                }
                self.end_scope();
                Ok(())
            }
        }
    }

    /// Resolves variable references in the given expression.
    fn resolve_expression(&mut self, expr: &ast::Expression) -> Result<(), String> {
        match &expr.expr {
            ast::ExpressionEnum::Primary(primary) => self.resolve_primary(primary),
            ast::ExpressionEnum::Unary(_, right) => self.resolve_expression(right),
            ast::ExpressionEnum::Factor(_, left, right)
            | ast::ExpressionEnum::Term(_, left, right)
            | ast::ExpressionEnum::Comparison(_, left, right)
            | ast::ExpressionEnum::Equality(_, left, right)
            | ast::ExpressionEnum::Logical(_, left, right) => {
                self.resolve_expression(left)?;
                self.resolve_expression(right)
            }
            ast::ExpressionEnum::Assignment(name, val) => {
                // Resolve the LHS identifier and RHS value.
                // resolve_primary handles recording the depth for the LHS identifier.
                self.resolve_primary(name)?;
                self.resolve_expression(val)
            }
            ast::ExpressionEnum::Call(callee, args) => {
                self.resolve_expression(callee)?;
                for arg in &args.args {
                    self.resolve_expression(arg)?;
                }
                Ok(())
            }
            // Field names are dynamic — only the object expression needs resolving.
            ast::ExpressionEnum::Get(object, _name) => self.resolve_expression(object),
            ast::ExpressionEnum::Set(object, _name, value) => {
                self.resolve_expression(object)?;
                self.resolve_expression(value)
            }
        }
    }

    /// Resolves variable references in the given primary expression.
    fn resolve_primary(&mut self, primary: &ast::Primary) -> Result<(), String> {
        match &primary.p {
            ast::PrimaryEnum::Literal(_) => Ok(()),
            ast::PrimaryEnum::Grouping(expr) => self.resolve_expression(expr),
            ast::PrimaryEnum::Identifier(name, id) => {
                // Walk the scope stack from innermost to outermost.
                // The reverse-enumeration index `i` is the hop count (depth).
                for (i, scope) in self.scopes.iter().rev().enumerate() {
                    match scope.get(name) {
                        // The variable is declared but its initializer has not finished
                        // evaluating yet — catch `var a = a;`.
                        Some(&false) => {
                            return Err(format!(
                                "[line {}:{}] Syntax error: can't read local variable '{}' in its own initializer.",
                                primary.span.start.line, primary.span.start.col, name
                            ));
                        }
                        // Variable is fully defined — record the depth and return.
                        Some(&true) => {
                            self.locals.insert(*id, i);
                            return Ok(());
                        }
                        // Not in this scope — continue searching outward.
                        None => {}
                    }
                }
                // Not found in any local scope — global variable. Record the depth as the
                // current scope stack size, which equals the number of hops from the current
                // runtime environment to the global environment.
                self.locals.insert(*id, self.scopes.len());
                Ok(())
            }
            ast::PrimaryEnum::This(id) => {
                if self.current_class == ClassType::None {
                    return Err(format!(
                        "[line {}:{}] Syntax error: Can't use 'this' outside of a class.",
                        primary.span.start.line, primary.span.start.col
                    ));
                }

                // Walk the scope stack from innermost to outermost to find the nearest enclosing
                // class scope.
                for (i, scope) in self.scopes.iter().rev().enumerate() {
                    if let Some(&true) = scope.get("this") {
                        // Record the depth for the `this` identifier and return.
                        self.locals.insert(*id, i);
                        return Ok(());
                    }
                }

                // Resolver can't find `this` despite having a Class context — this should never
                // happen if the resolver is correctly managing scopes, but we can return an error
                // just in case.
                Err(format!(
                    "[line {}:{}] Syntax error: Can't use 'this' outside of a class.",
                    primary.span.start.line, primary.span.start.col
                ))
            }
            ast::PrimaryEnum::SuperDot(_method_name, id) => {
                if self.current_class == ClassType::None {
                    return Err(format!(
                        "[line {}:{}] Syntax error: Can't use 'super' outside of a class.",
                        primary.span.start.line, primary.span.start.col
                    ));
                }
                if self.current_class != ClassType::Subclass {
                    return Err(format!(
                        "[line {}:{}] Syntax error: Can't use 'super' in a class with no superclass.",
                        primary.span.start.line, primary.span.start.col
                    ));
                }

                // Walk the scope stack from innermost to outermost to find the nearest scope
                // defining `super`.
                for (i, scope) in self.scopes.iter().rev().enumerate() {
                    if let Some(&true) = scope.get("super") {
                        // Record the depth for the `super` identifier and return.
                        self.locals.insert(*id, i);
                        return Ok(());
                    }
                }

                // Resolver can't find `super` despite having a Subclass context — this should never
                // happen if the resolver is correctly managing scopes, but we can return an error
                // just in case.
                Err(format!(
                    "[line {}:{}] Syntax error: Can't use 'super' outside of a class.",
                    primary.span.start.line, primary.span.start.col
                ))
            }
        }
    }
}

/// Resolves all variable references in the program and returns a mapping of
/// identifier IDs to their scope depth (number of hops up the environment chain).
/// Local variables map to their lexical depth; global variables map to `scopes.len()`
/// at the point of reference, which equals the number of hops to the global environment.
pub fn resolve(prog: &ast::Program) -> Result<LocalsType, String> {
    let mut resolver = Resolver::new(
        Vec::new(),
        HashMap::new(),
        FunctionType::None,
        ClassType::None,
    );
    resolver.resolve_program(prog)
}
