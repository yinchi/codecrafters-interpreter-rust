//! Module for abstract syntax tree (AST) generation according to the Lox grammar.

use std::{fmt::Debug, rc::Rc};

use super::tokenizer::Token;

pub mod ast;
pub mod span;
pub use ast::*;
pub use span::*;

/// The top-level AST node, which can be an expression, a statement, or a program.
pub enum ASTree {
    Expr(Expression),
    Declaration(Declaration),
    Program(Program),
}

impl Debug for ASTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // In all cases, just print the inner value.
        match self {
            ASTree::Expr(expr) => write!(f, "{:?}", expr),
            ASTree::Declaration(decl) => write!(f, "{:?}", decl),
            ASTree::Program(prog) => write!(f, "{:?}", prog),
        }
    }
}

/// Parses a primary expression, i.e. a literal or a parenthesized expression (group).
fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    match token.token_type.as_str() {
        "NUMBER" => {
            let n: f64 = token.literal.parse::<f64>().map_err(|_| {
                format!(
                    "[line {}:{}] Syntax error: invalid number literal '{}'.",
                    token.line, token.col, token.literal
                )
            })?;
            *pos += 1;
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::Number(n)), span.clone()).into())
        }
        "STRING" => {
            let s: String = token.literal.clone();
            *pos += 1;
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::String(s)), span.clone()).into())
        }
        "TRUE" => {
            *pos += 1;
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::True), span.clone()).into())
        }
        "FALSE" => {
            *pos += 1;
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::False), span.clone()).into())
        }
        "NIL" => {
            *pos += 1;
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::Nil), span.clone()).into())
        }
        // ( expr )
        "LEFT_PAREN" => {
            *pos += 1; // consume '('
            // Go back to the top-level parse function to parse the expression inside the
            // parentheses.
            let expr: Expression = parse_expression(tokens, pos)?;
            let next_token: &Token = tokens.get(*pos).ok_or_else(|| {
                let last = &tokens[(*pos).saturating_sub(1)];
                format!(
                    "[line {}:{}] Syntax error: expected ')' after expression, but found EOF.",
                    last.line, last.col
                )
            })?;
            if next_token.token_type != "RIGHT_PAREN" {
                return Err(format!(
                    "[line {}:{}] Syntax error: expected ')' after expression.",
                    next_token.line, next_token.col
                ));
            }
            *pos += 1; // consume ')'
            // Span covers from '(' to ')' inclusive.
            let span: Span = Span::new(
                Location::new(token.line, token.col),
                Location::new(next_token.line, next_token.col + next_token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Grouping(Box::new(expr)), span).into())
        }
        "IDENTIFIER" => {
            let name: String = token.lexeme.clone();
            let curr_pos: usize = *pos; // capture the current position for the identifier ID
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Identifier(name, curr_pos), span.clone()).into())
        }
        _ => Err(format!(
            "[line {}:{}] Syntax error: unexpected token '{}'.",
            token.line, token.col, token.lexeme
        )),
    }
}

/// Parse calls, or fall down one level (primary) if no call is found.
///
/// `call -> primary ( "(" [ arguments ] ")" )*`
fn parse_call(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut expr: Expression = parse_primary(tokens, pos)?;

    // Check for zero or more call suffixes after the primary expression.
    loop {
        if let Some(token) = tokens.get(*pos)
            && token.token_type == "LEFT_PAREN"
        {
            *pos += 1; // consume '('

            // Parse arguments if there are any.
            let args: Option<Arguments> = if let Some(next_token) = tokens.get(*pos)
                && next_token.token_type != "RIGHT_PAREN"
            {
                let mut args_vec: Vec<Expression> = Vec::new();
                loop {
                    args_vec.push(parse_expression(tokens, pos)?);
                    if let Some(comma_token) = tokens.get(*pos)
                        && comma_token.token_type == "COMMA"
                    {
                        *pos += 1; // consume ','
                    } else {
                        break;
                    }
                }
                Some(Arguments {
                    args: args_vec,
                    span: Span::new(
                        Location::new(token.line, token.col),
                        Location::new(tokens[*pos - 1].line, tokens[*pos - 1].col),
                    ),
                })
            } else {
                None
            };

            // Check for closing ')'
            let right_paren_token: &Token = tokens
                .get(*pos)
                .ok_or_else(|| "Unexpected end of input.".to_string())?;
            if right_paren_token.token_type != "RIGHT_PAREN" {
                return Err(format!(
                    "[line {}:{}] Syntax error: expected ')' after function call arguments.",
                    right_paren_token.line, right_paren_token.col
                ));
            }
            *pos += 1; // consume ')'

            let span: Span = Span::new(
                expr.span.start.clone(),
                Location::new(
                    right_paren_token.line,
                    right_paren_token.col + right_paren_token.lexeme.len(),
                ),
            );

            // Nest what we have so far in a Call (the loop allows for call chaining)
            expr = Expression::new(
                ExpressionEnum::Call(
                    Box::new(expr),
                    args.unwrap_or(Arguments {
                        args: Vec::new(),
                        span: span.clone(),
                    }),
                ),
                span.clone(),
            );
        } else {
            // No "(" means not a call, so break out of the loop and return whatever
            // we've parsed so far (which will be the primary expression if we never entered
            // the `if` branch).
            break;
        }
    }

    Ok(expr)
}

/// Parse unary expressions, or fall down one level (call) if no unary expression is found.
///
/// `unary -> ( "-" | "!" ) unary | call`
fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if matches!(token.token_type.as_str(), "MINUS" | "BANG") {
        let op: Operator = Operator {
            op: token.lexeme.clone(),
            span: Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            ),
        };

        *pos += 1;
        let operand: Expression = parse_unary(tokens, pos)?;
        let span = Span::new(
            Location::new(token.line, token.col),
            Location::new(operand.span.end.line, operand.span.end.col),
        );

        return Ok(Expression::new(
            ExpressionEnum::Unary(op, Box::new(operand)),
            span,
        ));
    }
    // No match => go to the next rule
    parse_call(tokens, pos)
}

/// Parse factor expressions, or fall down one level (unary) if no factor expression is found.
///
/// `factor -> unary ( ( "*" | "/" ) unary )*`
fn parse_factor(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_unary(tokens, pos)?;

    // Consume factors into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "STAR" | "SLASH") {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_unary(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );

            left = Expression::new(
                ExpressionEnum::Factor(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    // Once no more factors can be formed, return the left-hand side expression.
    Ok(left)
}

/// Parse term expressions, or fall down one level (factor) if no term expression is found.
///
/// `term -> factor ( ( "+" | "-" ) factor )*`
fn parse_term(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_factor(tokens, pos)?;

    // Consume terms into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "PLUS" | "MINUS") {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_factor(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );

            left = Expression::new(
                ExpressionEnum::Term(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    // Once no more terms can be formed, return the left-hand side expression.
    Ok(left)
}

/// Parse comparison expressions, or fall down one level (term) if no comparison operator is found.
///
/// `comparison -> term ( ( ">" | ">=" | "<" | "<=" ) term )*`
fn parse_comparison(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_term(tokens, pos)?;

    // Consume comparisons into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(
            token.token_type.as_str(),
            "GREATER" | "GREATER_EQUAL" | "LESS" | "LESS_EQUAL"
        ) {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_term(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );
            left = Expression::new(
                ExpressionEnum::Comparison(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    // Once no more comparisons can be formed, return the left-hand side expression.
    Ok(left)
}

/// Parse equality expressions, or fall down one level (comparison) if no equality operator is
/// found.
///
/// `equality -> comparison ( ( "==" | "!=" ) comparison )*`
fn parse_equality(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_comparison(tokens, pos)?;

    // Consume equality operators into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "EQUAL_EQUAL" | "BANG_EQUAL") {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_comparison(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );

            left = Expression::new(
                ExpressionEnum::Equality(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse a logic and expression, or fall down one level (equality) if no "and" operator is found.
///
/// `logic_and -> equality ( "and" equality )*`
fn parse_logic_and(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_equality(tokens, pos)?;

    // Consume "and" operators into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if token.token_type == "AND" {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_equality(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );
            left = Expression::new(
                ExpressionEnum::Logical(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse an or expression, or fall down one level (and) if no "or" operator is found.
///
/// `logic_or -> logic_and ( "or" logic_and )*`
fn parse_logic_or(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left: Expression = parse_logic_and(tokens, pos)?;

    // Consume "or" operators into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if token.token_type == "OR" {
            let op: Operator = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right: Expression = parse_logic_and(tokens, pos)?;
            let span: Span = Span::new(
                Location::new(left.span.start.line, left.span.start.col),
                Location::new(right.span.end.line, right.span.end.col),
            );
            left = Expression::new(
                ExpressionEnum::Logical(op, Box::new(left), Box::new(right)),
                span,
            );
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse an assignment expression, or fall down one level (logic_or) if no assignment operator is
/// present.
///
/// `assignment -> IDENTIFIER "=" assignment | logic_or`
fn parse_assignment(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let left: Expression = parse_logic_or(tokens, pos)?;

    if let Some(token) = tokens.get(*pos)
        && token.token_type == "EQUAL"
    {
        *pos += 1; // consume '='
        let right = parse_assignment(tokens, pos)?;

        // Check that LHS is an identifier.  If it is, build an Assignment expression.
        if let ExpressionEnum::Primary(p) = left.expr
            && let PrimaryEnum::Identifier(_, _id) = p.p
        {
            let span: Span = Span::new(left.span.start.clone(), right.span.end.clone());
            return Ok(Expression::new(
                ExpressionEnum::Assignment(p, Box::new(right)),
                span,
            ));
        } else {
            return Err(format!(
                "[line {}:{}] Invalid assignment target.",
                token.line, token.col
            ));
        }
    }
    Ok(left)
}

/// Parse an expression, which is just an assignment.
///
/// `expression     → assignment ;`
fn parse_expression(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    parse_assignment(tokens, pos)
}

/// Parse an expression statement, or fall down one level (expression) if no expression statement
/// is found.
///
/// `exprStmt -> expression ";"` (SEMICOLON token type)
fn parse_expr_stmt(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let expr: Expression = parse_expression(tokens, pos)?;

    // Check that the next token is a semicolon, and consume it.
    match tokens.get(*pos) {
        Some(token) if token.token_type == "SEMICOLON" => {
            *pos += 1; // consume the semicolon
            let span = Span::new(
                expr.span.start.clone(),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(ASTree::Declaration(Declaration::Statement(
                Statement::ExprStmt(ExprStmt::new(expr, span)),
                indent_depth,
            )))
        }
        _ => {
            // No match => go to the next rule (return the expr we already found)
            Ok(ASTree::Expr(expr))
        }
    }
}

/// Parse a print statement. Triggered only when we see "print", thus no fall-through.
///
/// `printStmt -> "print" exprStmt`
fn parse_print_stmt(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let print_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    *pos += 1; // consume 'print'

    let my_evaluable: ASTree = parse_expr_stmt(tokens, pos, indent_depth)?;
    // Check that expr is an expression statement
    if let ASTree::Declaration(Declaration::Statement(
        Statement::ExprStmt(expr_stmt),
        indent_depth,
    )) = my_evaluable
    {
        let span: Span = Span::new(
            Location::new(print_token.line, print_token.col),
            Location::new(expr_stmt.span.end.line, expr_stmt.span.end.col),
        );
        Ok(ASTree::Declaration(Declaration::Statement(
            Statement::PrintStmt(PrintStmt::new(expr_stmt, span)),
            indent_depth,
        )))
    } else {
        Err(format!(
            "[line {}:{}] Syntax error: expected expression statement after 'print'. Missing semicolon?",
            print_token.line, print_token.col
        ))
    }
}

/// Parse a return statement. Triggered only when we see "return", thus no fall-through.
///
/// `returnStmt     → "return" expression? ";" ;`
fn parse_return_stmt(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let return_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    *pos += 1; // consume 'return'

    // Check if the next token is a semicolon, which would mean no return value.
    let value: Option<Expression> = if let Some(token) = tokens.get(*pos)
        && token.token_type == "SEMICOLON"
    {
        None
    } else {
        Some(parse_expression(tokens, pos)?)
    };

    // Check for the semicolon after the optional expression.
    let semicolon_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if semicolon_token.token_type != "SEMICOLON" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ';' after return value.",
            semicolon_token.line, semicolon_token.col
        ));
    }
    *pos += 1; // consume ';'

    let span: Span = Span::new(
        Location::new(return_token.line, return_token.col),
        Location::new(
            semicolon_token.line,
            semicolon_token.col + semicolon_token.lexeme.len(),
        ),
    );

    Ok(ASTree::Declaration(Declaration::Statement(
        Statement::ReturnStmt(ReturnStmt { value, span }),
        indent_depth,
    )))
}

/// Parse a block statement.  Triggered only when we see `{`, thus no fall-through.
///
/// `block → "{" declaration* "}"`
fn parse_block(tokens: &[Token], pos: &mut usize, indent_depth: usize) -> Result<ASTree, String> {
    let open_brace_line: usize = tokens[*pos].line;
    let open_brace_col: usize = tokens[*pos].col;
    *pos += 1; // consume '{'

    let mut decls: Vec<Declaration> = Vec::new();
    loop {
        match tokens.get(*pos) {
            Some(token) if token.token_type == "RIGHT_BRACE" => {
                *pos += 1; // consume '}'
                return Ok(ASTree::Declaration(Declaration::Statement(
                    Statement::Block(decls),
                    indent_depth,
                )));
            }
            Some(token) if token.token_type == "EOF" => {
                return Err(format!(
                    "[line {}:{}] Syntax error: unmatched '{{'.",
                    open_brace_line, open_brace_col
                ));
            }
            None => {
                return Err(format!(
                    "[line {}:{}] Syntax error: unmatched '{{'.",
                    open_brace_line, open_brace_col
                ));
            }
            _ => match parse_declaration(tokens, pos, indent_depth + 1)? {
                ASTree::Declaration(decl) => decls.push(decl),
                ASTree::Expr(_) => {
                    // Bare expression found inside block.
                    // Did we get a closing brace or EOF instead of a semicolon after the expression?
                    let next_token = tokens
                        .get(*pos)
                        .ok_or_else(|| "Unexpected end of input.".to_string())?;
                    if next_token.token_type == "RIGHT_BRACE" {
                        return Err(format!(
                            "[line {}:{}] Syntax error: expected ';' after expression, but found '}}'.",
                            next_token.line, next_token.col
                        ));
                    } else if next_token.token_type == "EOF" {
                        return Err(format!(
                            "[line {}:{}] Syntax error: expected ';' after expression, but found EOF.",
                            next_token.line, next_token.col
                        ));
                    } else {
                        return Err(format!(
                            "[line {}:{}] Syntax error: unexpected '{}'.",
                            next_token.line, next_token.col, next_token.lexeme
                        ));
                    }
                }
                ASTree::Program(_) => unreachable!(),
            },
        }
    }
}

/// Parse an if statement. Triggered only when we see `if`, thus no fall-through.
///
/// `ifStmt         → "if" "(" expression ")" statement ( "else" statement )? ;`
fn parse_if_stmt(tokens: &[Token], pos: &mut usize, indent_depth: usize) -> Result<ASTree, String> {
    let if_pos: usize = *pos;
    *pos += 1; // consume 'if'

    // Check for '('
    let left_paren_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if left_paren_token.token_type != "LEFT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected '(' after 'if', but found '{}'.",
            left_paren_token.line, left_paren_token.col, left_paren_token.lexeme
        ));
    }
    *pos += 1; // consume '('

    let condition = parse_expression(tokens, pos)?;

    // Check for ')'
    let right_paren_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if right_paren_token.token_type != "RIGHT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ')' after if condition, but found '{}'.",
            right_paren_token.line, right_paren_token.col, right_paren_token.lexeme
        ));
    }
    *pos += 1; // consume ')'

    let then_branch: Box<Statement> = Box::new(
        match parse_statement(tokens, pos, indent_depth + 1)? {
            ASTree::Declaration(Declaration::Statement(stmt, _)) => stmt,
            ASTree::Declaration(Declaration::VarDecl(..)) => {
                return Err(format!(
                    "[line {}:{}] Syntax error: variable declaration not allowed directly inside 'if' statement. Use a block to create an inner scope.",
                    tokens[*pos - 1].line,
                    tokens[*pos - 1].col
                ));
            }
            ASTree::Declaration(Declaration::FunDecl(..)) => {
                return Err(format!(
                    "[line {}:{}] Syntax error: function declaration not allowed directly inside 'if' statement. Use a block to create an inner scope.",
                    tokens[*pos - 1].line,
                    tokens[*pos - 1].col
                ));
            }
            ASTree::Expr(_) => {
                return Err(format!(
                    "[line {}:{}] Syntax error: expected statement after if condition.",
                    tokens[*pos - 1].line,
                    tokens[*pos - 1].col
                ));
            }
            ASTree::Program(_) => unreachable!(),
        },
    );

    // Check for optional "else" branch and consume if present.
    let else_branch: Option<Box<Statement>> = if let Some(token) = tokens.get(*pos)
        && token.token_type == "ELSE"
    {
        *pos += 1; // consume 'else'
        Some(Box::new(
            match parse_statement(tokens, pos, indent_depth + 1)? {
                ASTree::Declaration(Declaration::Statement(stmt, _)) => stmt,
                ASTree::Declaration(Declaration::VarDecl(..)) => {
                    return Err(format!(
                        "[line {}:{}] Syntax error: variable declaration not allowed directly inside 'else' statement. Use a block to create an inner scope.",
                        tokens[*pos - 1].line,
                        tokens[*pos - 1].col
                    ));
                }
                ASTree::Declaration(Declaration::FunDecl(..)) => {
                    return Err(format!(
                        "[line {}:{}] Syntax error: function declaration not allowed directly inside 'else' statement. Use a block to create an inner scope.",
                        tokens[*pos - 1].line,
                        tokens[*pos - 1].col
                    ));
                }
                ASTree::Expr(_) => {
                    return Err(format!(
                        "[line {}:{}] Syntax error: expected statement after 'else'.",
                        tokens[*pos - 1].line,
                        tokens[*pos - 1].col
                    ));
                }
                ASTree::Program(_) => unreachable!(),
            },
        ))
    } else {
        None
    };

    // pos should now be past the end of the if statement,
    // so subtract 1 to get the end of the statement for the span.
    let span: Span = Span::new(
        Location::new(tokens[if_pos].line, tokens[if_pos].col),
        Location::new(tokens[*pos - 1].line, tokens[*pos - 1].col),
    );

    Ok(ASTree::Declaration(Declaration::Statement(
        Statement::IfStmt(IfStmt::new(
            condition,
            then_branch,
            else_branch,
            span,
            indent_depth,
        )),
        indent_depth,
    )))
}

/// Parse a while statement. Triggered only when we see `while`, thus no fall-through.
///
/// `whileStmt      → "while" "(" expression ")" statement ;`
fn parse_while_stmt(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let while_pos = *pos;
    *pos += 1; // consume 'while'

    // Check for '('
    let left_paren_token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if left_paren_token.token_type != "LEFT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected '(' after 'while', but found '{}'.",
            left_paren_token.line, left_paren_token.col, left_paren_token.lexeme
        ));
    }
    *pos += 1; // consume '('

    let condition = parse_expression(tokens, pos)?;

    // Check for ')'
    let right_paren_token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if right_paren_token.token_type != "RIGHT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ')' after while condition, but found '{}'.",
            right_paren_token.line, right_paren_token.col, right_paren_token.lexeme
        ));
    }
    *pos += 1; // consume ')'

    // Parse the body of the while loop as a statement.
    // Note that in Lox, any statement type is allowed as the loop body.
    let body = Box::new(match parse_statement(tokens, pos, indent_depth + 1)? {
        ASTree::Declaration(Declaration::Statement(stmt, _)) => stmt,
        ASTree::Declaration(Declaration::VarDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: variable declaration not allowed directly inside 'while' statement. Use a block to create an inner scope.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Declaration(Declaration::FunDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: function declaration not allowed directly inside 'while' statement. Use a block to create an inner scope.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Expr(_) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected statement after while condition.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Program(_) => unreachable!(),
    });

    // pos should now be past the end of the while statement,
    // so subtract 1 to get the end of the statement for the span.
    let span = Span::new(
        Location::new(tokens[while_pos].line, tokens[while_pos].col),
        Location::new(tokens[*pos - 1].line, tokens[*pos - 1].col),
    );

    Ok(ASTree::Declaration(Declaration::Statement(
        Statement::WhileStmt(WhileStmt::new(condition, body, span, indent_depth)),
        indent_depth,
    )))
}

/// Parse a for statement, applying desugaring to transform it into a while statement.
/// Triggered only when we see `for`, thus no fall-through.
///
/// Inserted into the AST as:
/// (block
///     (<dset! or set!> <name> <value>)?  ; loop variable declaration or assignment
///     (while
///         <condition or true(default)>   ; optional loop condition, e.g. true or (< i 10)
///         (block
///             (block <body>)             ; original loop body in its own nested scope
///             <increment>?               ; optional increment expression, e.g. (set! i (+ i 1))
///         )
///     )
/// )
///
/// `forStmt         → "for" "(" ( varDecl | exprStmt | ";" ) expression? ";" expression? ")" statement ;`
fn parse_for_stmt(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let for_pos = *pos;
    *pos += 1; // consume 'for'

    // Check for '('
    let left_paren_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if left_paren_token.token_type != "LEFT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected '(' after 'for', but found '{}'.",
            left_paren_token.line, left_paren_token.col, left_paren_token.lexeme
        ));
    }
    *pos += 1; // consume '('

    // Parse the initializer, which can be a variable declaration, an expression statement, or empty.
    let initializer: Option<ASTree> = if let Some(token) = tokens.get(*pos) {
        if token.token_type == "SEMICOLON" {
            *pos += 1; // consume ';'
            None
        } else if token.token_type == "VAR" {
            Some(parse_var_decl(tokens, pos, indent_depth + 1)?)
        } else {
            Some(parse_expr_stmt(tokens, pos, indent_depth + 1)?)
        }
    } else {
        return Err("Unexpected end of input.".to_string());
    };

    // Extract the initializer declaration if it exists, so we can put it in the desugared block.
    let init_decl: Option<Declaration> = match initializer {
        Some(ASTree::Declaration(decl)) => Some(decl),
        Some(ASTree::Expr(_)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected variable declaration or expression statement in for loop initializer.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        Some(ASTree::Program(_)) => unreachable!(),
        None => None,
    };

    // Parse the condition, which is an optional expression followed by a semicolon.
    let condition: Option<Expression> = if let Some(token) = tokens.get(*pos) {
        if token.token_type == "SEMICOLON" {
            None
        } else {
            Some(parse_expression(tokens, pos)?)
        }
    } else {
        return Err("Unexpected end of input.".to_string());
    };
    // Check for ';' after condition
    let semicolon_after_condition: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if semicolon_after_condition.token_type != "SEMICOLON" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ';' after for loop condition, but found '{}'.",
            semicolon_after_condition.line,
            semicolon_after_condition.col,
            semicolon_after_condition.lexeme
        ));
    }
    *pos += 1; // consume ';'

    // Parse the increment, which is an optional expression followed by a right parenthesis.
    let increment: Option<Expression> = if let Some(token) = tokens.get(*pos) {
        if token.token_type == "RIGHT_PAREN" {
            None
        } else {
            Some(parse_expression(tokens, pos)?)
        }
    } else {
        return Err("Unexpected end of input.".to_string());
    };
    // Check for ')' after increment
    let right_paren_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if right_paren_token.token_type != "RIGHT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ')' after for loop increment, but found '{}'.",
            right_paren_token.line, right_paren_token.col, right_paren_token.lexeme
        ));
    }
    *pos += 1; // consume ')'

    // Parse the body of the for loop as a statement.
    let body: Box<Statement> = Box::new(match parse_statement(tokens, pos, indent_depth + 1)? {
        ASTree::Declaration(Declaration::Statement(stmt, _)) => stmt,
        ASTree::Declaration(Declaration::VarDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: variable declaration not allowed directly inside 'for' statement body. Use a block to create an inner scope.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Declaration(Declaration::FunDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: function declaration not allowed directly inside 'for' statement body. Use a block to create an inner scope.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Expr(_) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected statement as body of 'for' loop.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Program(_) => unreachable!(),
    });

    // pos should now be past the end of the for statement,
    // so subtract 1 to get the end of the statement for the span.
    let span: Span = Span::new(
        Location::new(tokens[for_pos].line, tokens[for_pos].col),
        Location::new(tokens[*pos - 1].line, tokens[*pos - 1].col),
    );

    // We now have initializer as Option<ASTree>, condition as Option<Expression>,
    // increment as Option<Expression>, and body as Box<Statement>.
    // We need to desugar this into a while loop as described in the comment above.

    let desugared_body: Statement = if let Some(increment_expr) = increment {
        let new_span = increment_expr.span.clone();
        Statement::Block(vec![
            Declaration::Statement(*body, indent_depth + 1),
            Declaration::Statement(
                Statement::ExprStmt(ExprStmt::new(increment_expr, new_span)),
                indent_depth + 1,
            ),
        ])
    } else {
        // No increment, so the body stays the same.
        *body
    };

    let desugared_while: Statement = Statement::WhileStmt(WhileStmt::new(
        condition.unwrap_or_else(|| {
            Expression::new(
                ExpressionEnum::Primary(Primary::new(
                    PrimaryEnum::Literal(Literal::True),
                    span.clone(),
                )),
                // Use the entire FOR's span as the span for the implied `true` condition literal.
                span.clone(),
            )
        }),
        Box::new(desugared_body),
        span.clone(),
        indent_depth + 1,
    ));

    // If there is an initializer, the desugared for loop is a block containing the initializer.
    // Otherwise, it's a block containing just the desugared while loop.
    let desugared_for: Statement = if let Some(init) = init_decl {
        Statement::Block(vec![
            init,
            Declaration::Statement(desugared_while, indent_depth + 1),
        ])
    } else {
        Statement::Block(vec![Declaration::Statement(
            desugared_while,
            indent_depth + 1,
        )])
    };

    Ok(ASTree::Declaration(Declaration::Statement(
        desugared_for,
        indent_depth,
    )))
}

/// Parse a statement, which can be a block, print statement, while statement, or expression
/// statement. Fall-through to raw expressions from `parse_expr_stmt` if we see an expression
/// that isn't followed by a semicolon.
///
/// `statement → block | ifStmt | printStmt | returnStmt | whileStmt | exprStmt`
fn parse_statement(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if token.token_type == "LEFT_BRACE" {
        parse_block(tokens, pos, indent_depth)
    } else if token.token_type == "IF" {
        parse_if_stmt(tokens, pos, indent_depth)
    } else if token.token_type == "WHILE" {
        parse_while_stmt(tokens, pos, indent_depth)
    } else if token.token_type == "FOR" {
        parse_for_stmt(tokens, pos, indent_depth)
    } else if token.token_type == "PRINT" {
        parse_print_stmt(tokens, pos, indent_depth)
    } else if token.token_type == "RETURN" {
        parse_return_stmt(tokens, pos, indent_depth)
    } else {
        // No match for any other statement type, so try parsing an expression statement.
        parse_expr_stmt(tokens, pos, indent_depth)
    }
}

/// Parse a variable declaration. No fall-through, since variable declarations must start
/// with the "var" token.
///
/// `varDecl -> "var" IDENTIFIER ( "=" expression )? ";"`
fn parse_var_decl(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    // Handle "var" token
    let var_pos: usize = *pos;
    *pos += 1; // consume 'var'

    let name_token: &Token = tokens.get(*pos).ok_or_else(|| {
        let var_tok = &tokens[var_pos];
        format!(
            "[line {}:{}] Syntax error: expected variable name after 'var'.",
            var_tok.line, var_tok.col
        )
    })?;
    if name_token.token_type != "IDENTIFIER" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected variable name after 'var', but found '{}'.",
            name_token.line, name_token.col, name_token.lexeme
        ));
    }
    *pos += 1; // consume the variable name

    // Default values for the AST node fields, which will be updated if we find an initializer.
    let name: String = name_token.lexeme.clone();
    let mut initializer: Option<Expression> = None;

    // Check for an optional initializer. If there is an '=' token, consume it and
    // parse the initializer expression.
    if let Some(token) = tokens.get(*pos)
        && token.token_type == "EQUAL"
    {
        *pos += 1; // consume '='
        initializer = Some(parse_expression(tokens, pos)?);
    }

    // Handle ";"
    let semicolon_token: &Token = tokens.get(*pos).ok_or_else(|| {
        let last = &tokens[(*pos).saturating_sub(1)];
        format!(
            "[line {}:{}] Syntax error: expected ';' after variable declaration, but found EOF.",
            last.line, last.col
        )
    })?;
    if semicolon_token.token_type != "SEMICOLON" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected ';' after variable declaration, but found '{}'.",
            semicolon_token.line, semicolon_token.col, semicolon_token.lexeme
        ));
    }
    *pos += 1; // consume ';'

    // Build the AST node and return it.
    let span: Span = Span::new(
        Location::new(tokens[var_pos].line, tokens[var_pos].col),
        Location::new(
            semicolon_token.line,
            semicolon_token.col + semicolon_token.lexeme.len(),
        ),
    );
    Ok(ASTree::Declaration(Declaration::VarDecl(
        VarDecl::new(
            name,
            Location::new(name_token.line, name_token.col),
            initializer,
            span,
        ),
        indent_depth,
    )))
}

/// Parse a function declaration. No fall-through, since function declarations must start
/// with the "fun" token.
///
/// `funDecl -> "fun" IDENTIFIER "(" parameters? ")" block`
/// `parameters -> IDENTIFIER ( "," IDENTIFIER )*`
fn parse_fun_decl(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    // Handle "fun" token
    let fun_pos = *pos;
    *pos += 1; // consume 'fun'

    // Handle function name
    let name_token: &Token = tokens.get(*pos).ok_or_else(|| {
        let fun_tok = &tokens[fun_pos];
        format!(
            "[line {}:{}] Syntax error: expected function name after 'fun'.",
            fun_tok.line, fun_tok.col
        )
    })?;
    if name_token.token_type != "IDENTIFIER" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected function name after 'fun', but found '{}'.",
            name_token.line, name_token.col, name_token.lexeme
        ));
    }
    let name: String = name_token.lexeme.clone();
    *pos += 1; // consume the function name

    // Check for '('
    let left_paren_token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if left_paren_token.token_type != "LEFT_PAREN" {
        return Err(format!(
            "[line {}:{}] Syntax error: expected '(' after function name, but found '{}'.",
            left_paren_token.line, left_paren_token.col, left_paren_token.lexeme
        ));
    }
    *pos += 1; // consume '('

    // Parse parameters, which are zero or more identifiers separated by commas, followed by a ')'.
    let mut parameters: Vec<(String, Location)> = Vec::new();
    loop {
        if let Some(token) = tokens.get(*pos) {
            if token.token_type == "RIGHT_PAREN" {
                break; // end of parameter list
            } else if token.token_type == "IDENTIFIER" {
                parameters.push((token.lexeme.clone(), Location::new(token.line, token.col)));
                *pos += 1; // consume the identifier
                // If the next token is a comma, consume it and continue parsing parameters.
                if let Some(next_token) = tokens.get(*pos) {
                    if next_token.token_type == "COMMA" {
                        *pos += 1; // consume ','
                    } else if next_token.token_type == "RIGHT_PAREN" {
                        // end of parameter list, do nothing
                    } else {
                        return Err(format!(
                            "[line {}:{}] Syntax error: expected ',' or ')' after parameter in function declaration, but found '{}'.",
                            next_token.line, next_token.col, next_token.lexeme
                        ));
                    }
                } else {
                    let last = &tokens[(*pos).saturating_sub(1)];
                    return Err(format!(
                        "[line {}:{}] Syntax error: unexpected EOF in function parameter list.",
                        last.line, last.col
                    ));
                }
            } else {
                return Err(format!(
                    "[line {}:{}] Syntax error: expected parameter name or ')' in function declaration, but found '{}'.",
                    token.line, token.col, token.lexeme
                ));
            }
        } else {
            let last = &tokens[(*pos).saturating_sub(1)];
            return Err(format!(
                "[line {}:{}] Syntax error: unexpected EOF in function parameter list.",
                last.line, last.col
            ));
        }
    }
    *pos += 1; // consume ')'

    // Parse the function body, which is a block statement.
    let body: Statement = match parse_block(tokens, pos, indent_depth + 1)? {
        ASTree::Declaration(Declaration::Statement(stmt, _)) => {
            // Check that the statement is a block statement, since function bodies must be blocks.
            match stmt {
                Statement::Block(_) => stmt,
                _ => {
                    return Err(format!(
                        "[line {}:{}] Syntax error: found non-block statement as function body.",
                        tokens[*pos - 1].line,
                        tokens[*pos - 1].col
                    ));
                }
            }
        }
        ASTree::Declaration(Declaration::VarDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected block statement as function body, but found variable declaration.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Declaration(Declaration::FunDecl(..)) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected block statement as function body, but found function declaration.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Expr(_) => {
            return Err(format!(
                "[line {}:{}] Syntax error: expected block statement as function body, but found expression.",
                tokens[*pos - 1].line,
                tokens[*pos - 1].col
            ));
        }
        ASTree::Program(_) => unreachable!(),
    };

    let span: Span = Span::new(
        Location::new(tokens[fun_pos].line, tokens[fun_pos].col),
        Location::new(tokens[*pos - 1].line, tokens[*pos - 1].col),
    );
    Ok(ASTree::Declaration(Declaration::FunDecl(
        FunDecl::new(name, parameters, Rc::new(body), span),
        indent_depth,
    )))
}

/// Parse a declaration, which can be a variable declaration, a function declaration,
/// or a statement.
///
/// `declaration → fun Decl | varDecl | statement`
fn parse_declaration(
    tokens: &[Token],
    pos: &mut usize,
    indent_depth: usize,
) -> Result<ASTree, String> {
    let token: &Token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if token.token_type == "VAR" {
        parse_var_decl(tokens, pos, indent_depth)
    } else if token.token_type == "FUN" {
        parse_fun_decl(tokens, pos, indent_depth)
    } else {
        parse_statement(tokens, pos, indent_depth)
    }
}

/// Sentinel returned by `parse_program` when the input is not a program (no declarations
/// found, and `parse_declaration` returned a bare expression rather than an `Err`).
/// Signals to `parse()` that it should retry as a bare-expression parse.
const NOT_A_PROGRAM: &str = "__NOT_A_PROGRAM__";

/// Parse a program, which is a sequence of declarations followed by an EOF token.
/// Output can then be fed into `your_program.sh run`.
fn parse_program(tokens: &[Token], pos: &mut usize) -> Result<ASTree, String> {
    let mut declarations: Vec<Declaration> = Vec::new();

    // Consume declarations (check that each new Evaluable is a declaration)
    // until we hit EOF.  If we run out of tokens or there are tokens after EOF, the
    // program is malformed.
    while let Some(token) = tokens.get(*pos) {
        if token.token_type == "EOF" {
            *pos += 1; // consume EOF
            let span: Span = Span::new(
                Location::new(1, 1),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            return Ok(ASTree::Program(Program::new(declarations, span)));
        }
        // Since not EOF, consume a declaration.  Note scope-depth is 1-based to
        // match indentation of top-level declarations in parser output (inside `prog`).
        match parse_declaration(tokens, pos, 1)? {
            ASTree::Declaration(decl) => declarations.push(decl),
            ASTree::Expr(_) => {
                // parse_declaration returned Ok(Expr)
                // No declarations found => not a program, signal to `parse` to try parsing entire
                // input as an expression instead (but we must remove the EOF first).
                if declarations.is_empty() {
                    return Err(NOT_A_PROGRAM.to_string());
                }
                return Err(format!(
                    "[line {}:{}] Syntax error: expected declaration at top level.",
                    token.line, token.col
                ));
            }
            ASTree::Program(_) => {
                return Err(format!(
                    "[line {}:{}] Syntax error: unexpected 'program' declaration.",
                    token.line, token.col
                ));
            }
        }
    }
    Err("Unexpected end of input.".to_string())
}

/// The top-level parse function, which will be called from main.rs.
pub fn parse(tokens: &[Token]) -> Result<ASTree, String> {
    let mut pos = 0;
    // Try to parse a full program first. If that fails, try remove the EOF token and
    // parse a single expression.  We do not consider the statement level since a single statement
    // with EOF is already a valid program.
    match parse_program(tokens, &mut pos) {
        Ok(ast) => Ok(ast),
        Err(e) if e == NOT_A_PROGRAM => {
            // Try parsing an expression statement without the EOF token.
            let tokens_without_eof: &[Token] = if let Some(last_token) = tokens.last() {
                if last_token.token_type == "EOF" {
                    &tokens[..tokens.len() - 1]
                } else {
                    tokens
                }
            } else {
                tokens
            };
            let mut expr_pos: usize = 0;
            let expr: Expression = parse_expression(tokens_without_eof, &mut expr_pos)?;
            if let Some(leftover) = tokens_without_eof.get(expr_pos) {
                return Err(format!(
                    "[line {}:{}] Syntax error: unexpected token '{}'.",
                    leftover.line, leftover.col, leftover.lexeme
                ));
            }
            Ok(ASTree::Expr(expr))
        }
        Err(e) => Err(e),
    }
}
