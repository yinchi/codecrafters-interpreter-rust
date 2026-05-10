use std::fmt::{Debug, Display};
use std::rc::Rc;

use super::*;

/// Indent a multiline string by the given number of spaces.
fn indent_string(s: &str, indent: usize) -> String {
    let indent_str = " ".repeat(indent);
    s.lines()
        .map(|line| format!("{}{}", indent_str, line))
        .collect::<Vec<String>>()
        .join("\n")
}

impl From<Literal> for PrimaryEnum {
    /// Cast from Literal to PrimaryEnum.
    fn from(l: Literal) -> Self {
        PrimaryEnum::Literal(l)
    }
}

impl From<Primary> for ExpressionEnum {
    /// Cast from Primary to ExpressionEnum.
    fn from(p: Primary) -> Self {
        ExpressionEnum::Primary(p)
    }
}

impl From<ExpressionEnum> for Expression {
    /// Cast from ExpressionEnum to Expression, inferring the span of the full expression from the
    /// spans of the sub-expressions.
    fn from(e_enum: ExpressionEnum) -> Self {
        let span = match &e_enum {
            ExpressionEnum::Primary(primary) => primary.span.clone(),
            ExpressionEnum::Unary(op, operand) => {
                // Full span is `<op> <operand>`
                Span::new(op.span.start.clone(), operand.span.end.clone())
            }

            // For the following, full span is `<left> <_op> <right>`
            ExpressionEnum::Factor(_op, left, right)
            | ExpressionEnum::Term(_op, left, right)
            | ExpressionEnum::Comparison(_op, left, right)
            | ExpressionEnum::Equality(_op, left, right)
            | ExpressionEnum::Logical(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }

            // Full span is `<name> = <value>`
            ExpressionEnum::Assignment(name, value) => {
                Span::new(name.span.start.clone(), value.span.end.clone())
            }

            // Full span is `<callee>(<arg1>, <arg2>, ...)`
            ExpressionEnum::Call(callee, args) => {
                Span::new(callee.span.start.clone(), args.span.end.clone())
            }

            // Full span is `<object>.<property>`
            ExpressionEnum::Get(object, _property) => {
                Span::new(object.span.start.clone(), object.span.end.clone())
            }

            // Full span is `<object>.<property> = <value>`
            ExpressionEnum::Set(object, _property, value) => {
                Span::new(object.span.start.clone(), value.span.end.clone())
            }
        };
        Expression::new(e_enum, span)
    }
}

impl From<Primary> for Expression {
    /// Cast from Primary -> ExpressionEnum -> Expression.
    fn from(p: Primary) -> Self {
        Self::from(ExpressionEnum::from(p))
    }
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::Number(n1), Literal::Number(n2)) => n1 == n2,
            (Literal::String(s1), Literal::String(s2)) => s1 == s2,

            (Literal::True, Literal::True)
            | (Literal::False, Literal::False)
            | (Literal::Nil, Literal::Nil) => true,

            // Class identity by Rc pointer equality
            (Literal::Class(c1), Literal::Class(c2)) => Rc::ptr_eq(&c1.id, &c2.id),

            // User callables are equal if they have the same function declaration (by Rc identity).
            (Literal::UserCallable(f1), Literal::UserCallable(f2)) => {
                Rc::ptr_eq(&f1.decl, &f2.decl)
            }

            // Native callables are equal if they have the same name (since they reference the same built-in function).
            (Literal::NativeCallable(f1), Literal::NativeCallable(f2)) => f1.name == f2.name,

            // Compare instance identity by Rc pointer equality
            (Literal::Instance(i1), Literal::Instance(i2)) => Rc::ptr_eq(i1, i2),
            _ => false, // Different variants are not equal
        }
    }
}

impl Display for Literal {
    /** Called with printing a `Literal` with the {} formatter. Displays integers as-is (no ".0").
    Use the `Debug` implementation (with ".0") for the parse step and `Display` for the
    evaluation/execution steps. */
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "{}", s),
            Literal::Class(class) => write!(f, "{}", class.name),
            Literal::True => write!(f, "true"),
            Literal::False => write!(f, "false"),
            Literal::Nil => write!(f, "nil"),
            Literal::UserCallable(callable) => write!(f, "<fn {}>", callable.decl.name),
            Literal::NativeCallable(callable) => write!(f, "<native fn {}>", callable.name),
            Literal::Instance(instance) => write!(f, "{} instance", instance.borrow().class.name),
        }
    }
}

impl Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.op)
    }
}

impl Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        /* Prints:

        (prog
          <decl1>
          <decl2>
          ...
        )

        */
        result.push_str("(prog\n");
        for decl in &self.declarations {
            let decl_str = format!("{:?}", decl);
            result.push_str(&format!("{}\n", indent_string(&decl_str, 2)));
        }
        result.push_str(")\n");
        write!(f, "{}", result)
    }
}

impl Debug for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Declaration::ClassDecl(class_decl) => {
                write!(f, "{:?}", class_decl)
            }
            Declaration::FunDecl(fun_decl) => {
                write!(f, "{:?}", fun_decl)
            }
            Declaration::VarDecl(var_decl) => {
                write!(f, "{:?}", var_decl)
            }
            Declaration::Statement(stmt) => {
                write!(f, "{:?}", stmt)
            }
        }
    }
}

impl Debug for ClassDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Opening `(class <name>`
        let mut result = if let Some((superclass_name, _)) = &self.superclass {
            // (subclass <name> <superclass> ...)
            format!("(subclass {} {}", self.name, superclass_name)
        } else {
            // (class <name> ...)
            format!("(class {}", self.name)
        };

        for method in &self.methods {
            // Each method on a new line, indented by 2 spaces.
            let method_str = format!("{:?}", method);
            result.push_str(format!("\n{}", indent_string(&method_str, 2)).as_str());
        }

        // Newline before closing `)` if there are methods.
        if !self.methods.is_empty() {
            result.push('\n');
        }

        // Closing `)`.
        result.push(')');

        write!(f, "{}", result)
    }
}

impl Debug for FunDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params_str = self
            .params
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let block_str = format!("{:?}", self.body);
        let indented_block_str = indent_string(&block_str, 2);
        write!(
            f,
            "(fn! {} ({})\n{}\n)",
            self.name, params_str, indented_block_str
        )
    }
}

impl Debug for VarDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.initializer {
            // dassign! if there's an initializer, otherwise just decl!.
            Some(expr) => write!(f, "(dassign! {} {:?})", self.name, expr),
            None => write!(f, "(decl! {})", self.name),
        }
    }
}

impl Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Just print the inner expression.
            Statement::ExprStmt(expr_stmt) => write!(f, "{:?}", expr_stmt),
            Statement::PrintStmt(print_stmt) => write!(f, "{:?}", print_stmt),
            Statement::ReturnStmt(return_stmt) => write!(f, "{:?}", return_stmt),

            // The cases below may involve blocks (possibly multiple lines), so put each
            // child on a new line and increase indentation by 2. For example:
            //
            // (if (== outer 1)
            //   (print "outer is 1")
            //   (if (== nested 2)
            //     (print (+ "outer is not 1" "and nested is 2"))
            //     (print (+ "outer is not 1" "and nested is not 2"))
            //   )
            // )
            Statement::IfStmt(if_stmt) => {
                let mut result = format!("(if {:?}", if_stmt.condition);
                let then_branch_str = format!("{:?}", if_stmt.then_branch);
                let indented_then_branch_str = indent_string(&then_branch_str, 2);
                result.push_str(&format!("\n{}", indented_then_branch_str));

                if let Some(else_branch) = &if_stmt.else_branch {
                    let else_branch_str = format!("{:?}", else_branch);
                    let indented_else_branch_str = indent_string(&else_branch_str, 2);
                    result.push_str(&format!("\n{}", indented_else_branch_str));
                }
                result.push_str("\n)");
                write!(f, "{}", result)
            }
            Statement::WhileStmt(while_stmt) => {
                let result = format!(
                    "(while {:?}\n{:?}\n)",
                    while_stmt.condition,
                    indent_string(&format!("{:?}", while_stmt.body), 2)
                );
                write!(f, "{}", result)
            }
            Statement::Block(decls) => {
                let mut result = String::from("(block");

                // Push each declaration in the block on a new line, indented by 2 spaces per
                //indent depth.
                for decl in decls {
                    let decl_str = format!("{:?}", decl);
                    result.push_str(&format!("\n{}", indent_string(&decl_str, 2)));
                }

                // Newline before closing `)` if there are declarations.
                if !decls.is_empty() {
                    result.push('\n');
                }

                result.push(')');
                write!(f, "{}", result)
            }
        }
    }
}

impl Debug for PrintStmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(print {:?})", self.expr_stmt)
    }
}

impl Debug for ReturnStmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(expr) => write!(f, "(return {:?})", expr),
            None => write!(f, "(return)"),
        }
    }
}

impl Debug for ExprStmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.expr)
    }
}

impl Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.expr)
    }
}

impl Debug for ExpressionEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionEnum::Primary(primary) => write!(f, "{:?}", primary),
            ExpressionEnum::Unary(op, expr) => write!(f, "({:?} {:?})", op, expr),
            ExpressionEnum::Factor(op, left, right)
            | ExpressionEnum::Term(op, left, right)
            | ExpressionEnum::Comparison(op, left, right)
            | ExpressionEnum::Equality(op, left, right)
            | ExpressionEnum::Logical(op, left, right) => {
                write!(f, "({:?} {:?} {:?})", op, left, right)
            }
            ExpressionEnum::Assignment(name, value) => {
                write!(f, "(assign! {:?} {:?})", name, value)
            }
            ExpressionEnum::Call(callee, args) => {
                let mut result: String = format!("({:?}", callee);
                for arg in &args.args {
                    result.push_str(&format!(" {:?}", arg));
                }
                result.push(')');
                write!(f, "{}", result)
            }
            ExpressionEnum::Get(object, property) => write!(f, "(get {:?} {:?})", object, property),
            ExpressionEnum::Set(object, property, value) => {
                write!(f, "(set! {:?} {:?} {:?})", object, property, value)
            }
        }
    }
}

impl Debug for Primary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.p)
    }
}

impl Debug for PrimaryEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryEnum::Literal(lit) => write!(f, "{:?}", lit),
            PrimaryEnum::Grouping(expr) => write!(f, "(group {:?})", expr),
            PrimaryEnum::Identifier(name, _id) => write!(f, "{}", name),
            PrimaryEnum::This(_id) => write!(f, "this"),
            PrimaryEnum::SuperDot(method_name, _id) => write!(f, "super.{}", method_name),
        }
    }
}

impl Debug for Literal {
    /** Called when printing a `Literal` with the {:?} formatter.
    Formats numbers with at least one decimal place as expected for the parsing step.
    Use the `Debug` implementation (with ".0") for the parse step and `Display` for the
    evaluation/execution steps. */
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(n) => {
                if n.fract() == 0.0 {
                    // Show at least one decimal place
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Literal::String(s) => write!(f, "{}", s),
            Literal::Class(class) => write!(f, "{}", class.name),
            Literal::True => write!(f, "true"),
            Literal::False => write!(f, "false"),
            Literal::Nil => write!(f, "nil"),
            Literal::UserCallable(callable) => write!(f, "<fn {}>", callable.decl.name),
            Literal::NativeCallable(callable) => write!(f, "<native fn {}>", callable.name),
            Literal::Instance(instance) => write!(f, "<{} instance>", instance.borrow().class.name),
        }
    }
}
