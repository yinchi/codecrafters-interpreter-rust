use std::fmt::{Debug, Display};

use super::*;

impl Program {
    pub fn new(declarations: Vec<Declaration>, span: Span) -> Self {
        Self { declarations, span }
    }
}

impl VarDecl {
    pub fn new(name: String, initializer: Option<Expression>, span: Span) -> Self {
        Self {
            name,
            initializer,
            span,
        }
    }
}

impl PrintStmt {
    pub fn new(expr_stmt: ExprStmt, span: Span) -> Self {
        Self { expr_stmt, span }
    }
}

impl ExprStmt {
    pub fn new(expr: Expression, span: Span) -> Self {
        Self { expr, span }
    }
}

impl IfStmt {
    pub fn new(
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
        span: Span,
        indent_depth: usize,
    ) -> Self {
        Self {
            condition,
            then_branch,
            else_branch,
            span,
            indent_depth,
        }
    }
}

impl WhileStmt {
    pub fn new(
        condition: Expression,
        body: Box<Statement>,
        span: Span,
        indent_depth: usize,
    ) -> Self {
        Self {
            condition,
            body,
            span,
            indent_depth,
        }
    }
}

impl Expression {
    pub fn new(expr: ExpressionEnum, span: Span) -> Self {
        Self { expr, span }
    }
}

impl Primary {
    pub fn new(primary: PrimaryEnum, span: Span) -> Self {
        Self { p: primary, span }
    }
}

impl From<Literal> for PrimaryEnum {
    fn from(l: Literal) -> Self {
        PrimaryEnum::Literal(l)
    }
}

// From PrimaryEnum to Primary requires a span, so we need an explict new().

impl From<Primary> for ExpressionEnum {
    fn from(p: Primary) -> Self {
        ExpressionEnum::Primary(p)
    }
}

impl From<ExpressionEnum> for Expression {
    fn from(e_enum: ExpressionEnum) -> Self {
        let span = match &e_enum {
            ExpressionEnum::Primary(primary) => primary.span.clone(),
            ExpressionEnum::Unary(op, operand) => {
                // Full span is `<op> <operand>`
                Span::new(op.span.start.clone(), operand.span.end.clone())
            }

            // For the following, full span is `<left> <_op> <right>`
            ExpressionEnum::Factor(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }
            ExpressionEnum::Term(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }
            ExpressionEnum::Comparison(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }
            ExpressionEnum::Equality(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }
            ExpressionEnum::Logical(_op, left, right) => {
                Span::new(left.span.start.clone(), right.span.end.clone())
            }

            // Full span is `<name> = <value>`
            ExpressionEnum::Assignment(name, value) => {
                Span::new(name.span.start.clone(), value.span.end.clone())
            }
        };
        Expression::new(e_enum, span)
    }
}

impl From<Primary> for Expression {
    // Cast from Primary -> ExpressionEnum -> Expression, using the From impls defined above.
    fn from(p: Primary) -> Self {
        Self::from(ExpressionEnum::from(p))
    }
}

impl Display for Literal {
    /// Called with printing a Literal with the {} formatter. Displays integers as-is (no ".0").
    /// Use the Debug implementation (with ".0") for the parse step and Display for the
    /// evaluation/execution steps.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "{}", s),
            Literal::True => write!(f, "true"),
            Literal::False => write!(f, "false"),
            Literal::Nil => write!(f, "nil"),
        }
    }
}

impl Clone for Literal {
    fn clone(&self) -> Self {
        match self {
            Literal::Number(n) => Literal::Number(*n),
            Literal::String(s) => Literal::String(s.clone()),
            Literal::True => Literal::True,
            Literal::False => Literal::False,
            Literal::Nil => Literal::Nil,
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
        // Print:
        // (prog
        //   stmt1
        //   stmt2
        //   ...
        // )
        result.push_str("(prog\n");
        for decl in &self.declarations {
            result.push_str(&format!("{:?}\n", decl));
        }
        result.push_str(")\n");
        write!(f, "{}", result)
    }
}

impl Debug for Declaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Declaration::VarDecl(var_decl, indent_depth) => {
                write!(f, "{}{:?}", " ".repeat(2 * indent_depth), var_decl)
            }
            Declaration::Statement(stmt, indent_depth) => {
                write!(f, "{}{:?}", " ".repeat(2 * indent_depth), stmt)
            }
        }
    }
}

impl Debug for VarDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.initializer {
            // dset! if there's an initializer, otherwise just decl!.
            Some(expr) => write!(f, "(dset! {} {:?})", self.name, expr),
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
                let branch_indent = " ".repeat(2 * (if_stmt.indent_depth + 1));
                let close_indent = " ".repeat(2 * if_stmt.indent_depth);
                let mut result = format!(
                    "(if {:?}\n{}{:?}",
                    if_stmt.condition, branch_indent, if_stmt.then_branch
                );
                if let Some(else_branch) = &if_stmt.else_branch {
                    result.push_str(&format!("\n{}{:?}", branch_indent, else_branch));
                }
                result.push_str(&format!("\n{})", close_indent));
                write!(f, "{}", result)
            }
            Statement::WhileStmt(while_stmt) => {
                let branch_indent = " ".repeat(2 * (while_stmt.indent_depth + 1));
                let close_indent = " ".repeat(2 * while_stmt.indent_depth);
                let result = format!(
                    "(while {:?}\n{}{:?}\n{})",
                    while_stmt.condition, branch_indent, while_stmt.body, close_indent
                );
                write!(f, "{}", result)
            }
            Statement::Block(decls) => {
                let mut result = String::from("(block");

                // Push each declaration in the block on a new line, indented by 2 spaces per
                //indent depth.
                for decl in decls {
                    result.push_str(&format!("\n{:?}", decl));
                }

                // Infer the indent depth of the block from the first declaration (if any) to
                // determine how much to indent the closing parenthesis.
                if let Some(first_decl) = decls.first() {
                    let inner_depth = match first_decl {
                        Declaration::VarDecl(_, d) | Declaration::Statement(_, d) => *d,
                    };
                    result.push_str(&format!(
                        "\n{})",
                        " ".repeat(2 * inner_depth.saturating_sub(1))
                    ));
                } else {
                    // No declarations, entire string is just "(block)" with no newlines.
                    result.push(')');
                }
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
            ExpressionEnum::Assignment(name, value) => write!(f, "(set! {:?} {:?})", name, value),
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
            PrimaryEnum::Identifier(name) => write!(f, "{}", name),
        }
    }
}

impl Debug for Literal {
    /// Called when printing a Literal with the {:?} formatter.
    /// Formats numbers with at least one decimal place as expected for the parsing step.
    /// Use the Debug implementation (with ".0") for the parse step and Display for the
    /// evaluation/execution steps.
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
            Literal::True => write!(f, "true"),
            Literal::False => write!(f, "false"),
            Literal::Nil => write!(f, "nil"),
        }
    }
}
