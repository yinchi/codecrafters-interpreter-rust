//! Module for abstract syntax tree (AST) generation according to the Lox grammar.

use std::fmt::Debug;

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
    let token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    match token.token_type.as_str() {
        "NUMBER" => {
            let n = token.literal.parse::<f64>().map_err(|_| {
                format!(
                    "[line {}:{}] Syntax error: invalid number literal '{}'.",
                    token.line, token.col, token.literal
                )
            })?;
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::Number(n)), span.clone()).into())
        }
        "STRING" => {
            let s = token.literal.clone();
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::String(s)), span.clone()).into())
        }
        "TRUE" => {
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::True), span.clone()).into())
        }
        "FALSE" => {
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Literal(Literal::False), span.clone()).into())
        }
        "NIL" => {
            *pos += 1;
            let span = Span::new(
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
            let expr = parse_expression(tokens, pos)?;
            let next_token = tokens
                .get(*pos)
                .ok_or_else(|| "Syntax error: Unexpected end of input.".to_string())?;
            if next_token.token_type != "RIGHT_PAREN" {
                return Err(format!(
                    "[line {}:{}] Syntax error: expected ')' after expression.",
                    next_token.line, next_token.col
                ));
            }
            *pos += 1; // consume ')'
            let span = Span::new(
                expr.span.start.clone(),
                // For the end location, use the position after the `next_token` and closing parenthesis.
                Location::new(
                    next_token.line,
                    next_token.col + next_token.lexeme.len() + 1,
                ),
            );
            Ok(Primary::new(PrimaryEnum::Grouping(Box::new(expr)), span.clone()).into())
        }
        "IDENTIFIER" => {
            let name = token.lexeme.clone();
            *pos += 1;
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            );
            Ok(Primary::new(PrimaryEnum::Identifier(name), span.clone()).into())
        }
        _ => Err(format!(
            "[line {}:{}] Syntax error: unexpected token '{}'.",
            token.line, token.col, token.lexeme
        )),
    }
}

/// Parse unary expressions, or fall down one level (primary) if no unary expression is found.
///
/// `unary -> ( "-" | "!" ) unary`
fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if matches!(token.token_type.as_str(), "MINUS" | "BANG") {
        let op = Operator {
            op: token.lexeme.clone(),
            span: Span::new(
                Location::new(token.line, token.col),
                Location::new(token.line, token.col + token.lexeme.len()),
            ),
        };

        *pos += 1;
        let operand = parse_unary(tokens, pos)?;
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
    parse_primary(tokens, pos)
}

/// Parse factor expressions, or fall down one level (unary) if no factor expression is found.
///
/// `factor -> unary ( ( "*" | "/" ) unary )*`
fn parse_factor(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let mut left = parse_unary(tokens, pos)?;

    // Consume factors into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "STAR" | "SLASH") {
            let op = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right = parse_unary(tokens, pos)?;
            let span = Span::new(
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
    let mut left = parse_factor(tokens, pos)?;

    // Consume terms into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "PLUS" | "MINUS") {
            let op = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right = parse_factor(tokens, pos)?;
            let span = Span::new(
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
    let mut left = parse_term(tokens, pos)?;

    // Consume comparisons into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(
            token.token_type.as_str(),
            "GREATER" | "GREATER_EQUAL" | "LESS" | "LESS_EQUAL"
        ) {
            let op = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right = parse_term(tokens, pos)?;
            let span = Span::new(
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
    let mut left = parse_comparison(tokens, pos)?;

    // Consume equality operators into `left` until we can't anymore.
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.token_type.as_str(), "EQUAL_EQUAL" | "BANG_EQUAL") {
            let op = Operator {
                op: token.lexeme.clone(),
                span: Span::new(
                    Location::new(token.line, token.col),
                    Location::new(token.line, token.col + token.lexeme.len()),
                ),
            };

            *pos += 1;
            let right = parse_comparison(tokens, pos)?;
            let span = Span::new(
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

/// Parse an assignment expression, or fall down one level (equality) if no assignment operator is
/// present.
///
/// `assignment -> IDENTIFIER "=" assignment | equality`
fn parse_assignment(tokens: &[Token], pos: &mut usize) -> Result<Expression, String> {
    let left = parse_equality(tokens, pos)?;

    if let Some(token) = tokens.get(*pos)
        && token.token_type == "EQUAL"
    {
        *pos += 1; // consume '='
        let right = parse_assignment(tokens, pos)?;

        // Check that LHS is an identifier.  If it is, build an Assignment expression.
        if let ExpressionEnum::Primary(p) = left.expr
            && let PrimaryEnum::Identifier(_) = p.p
        {
            let span = Span::new(left.span.start.clone(), right.span.end.clone());
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
    scope_depth: usize,
) -> Result<ASTree, String> {
    let expr = parse_expression(tokens, pos)?;

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
                scope_depth,
            )))
        }
        _ => {
            // No match => go to the next rule (return the expr we already found)
            Ok(ASTree::Expr(expr))
        }
    }
}

/// Parse a print statement, or fall down one level (expression statement) if no print statement is
/// found.
///
/// `printStmt -> "print" exprStmt`
fn parse_print_stmt(
    tokens: &[Token],
    pos: &mut usize,
    scope_depth: usize,
) -> Result<ASTree, String> {
    // `print` acts like a unary operator, so reuse the unary parsing logic to consume it
    // if it's there.
    let token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if token.token_type == "PRINT" {
        *pos += 1; // consume 'print'
        let my_evaluable = parse_expr_stmt(tokens, pos, scope_depth)?;
        // Check that expr is an expression statement
        if let ASTree::Declaration(Declaration::Statement(
            Statement::ExprStmt(expr_stmt),
            scope_depth,
        )) = my_evaluable
        {
            let span = Span::new(
                Location::new(token.line, token.col),
                Location::new(expr_stmt.span.end.line, expr_stmt.span.end.col),
            );
            Ok(ASTree::Declaration(Declaration::Statement(
                Statement::PrintStmt(PrintStmt::new(expr_stmt, span)),
                scope_depth,
            )))
        } else {
            Err(format!(
                "[line {}:{}] Syntax error: expected expression statement after 'print'. Missing semicolon?",
                token.line, token.col
            ))
        }
    } else {
        // No match => go to the next rule
        parse_expr_stmt(tokens, pos, scope_depth)
    }
}

/// Parse a block statement.
///
/// `block → "{" declaration* "}"`
fn parse_block(tokens: &[Token], pos: &mut usize, scope_depth: usize) -> Result<ASTree, String> {
    let open_brace_line = tokens[*pos].line;
    let open_brace_col = tokens[*pos].col;
    *pos += 1; // consume '{'

    let mut decls: Vec<Declaration> = Vec::new();
    loop {
        match tokens.get(*pos) {
            Some(token) if token.token_type == "RIGHT_BRACE" => {
                *pos += 1; // consume '}'
                return Ok(ASTree::Declaration(Declaration::Statement(
                    Statement::Block(decls),
                    scope_depth,
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
            _ => match parse_declaration(tokens, pos, scope_depth + 1)? {
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

/// Parse a statement, which can be a block, print statement, or expression statement.
/// Expression statements are detected by falling through from `parse_print_stmt` if no "print"
/// token is found.
///
/// `statement → block | printStmt | exprStmt`
fn parse_statement(
    tokens: &[Token],
    pos: &mut usize,
    scope_depth: usize,
) -> Result<ASTree, String> {
    let token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if token.token_type == "LEFT_BRACE" {
        parse_block(tokens, pos, scope_depth)
    } else {
        parse_print_stmt(tokens, pos, scope_depth)
    }
}

/// Parse a variable declaration, or fall down one level (statement) if no variable
/// declaration is found.
///
/// `varDecl -> "var" IDENTIFIER ( "=" expression )? ";"`
fn parse_var_decl(tokens: &[Token], pos: &mut usize, scope_depth: usize) -> Result<ASTree, String> {
    let token = tokens
        .get(*pos)
        .ok_or_else(|| "Unexpected end of input.".to_string())?;
    if token.token_type == "VAR" {
        // Handle "var" token
        *pos += 1; // consume 'var'
        let name_token = tokens
            .get(*pos)
            .ok_or_else(|| "Syntax error: expected variable name after 'var'.".to_string())?;
        if name_token.token_type != "IDENTIFIER" {
            return Err(format!(
                "[line {}:{}] Syntax error: expected variable name after 'var', but found '{}'.",
                name_token.line, name_token.col, name_token.lexeme
            ));
        }
        *pos += 1; // consume the variable name

        // Default values for the AST node fields, which will be updated if we find an initializer.
        let name = name_token.lexeme.clone();
        let mut initializer = None;

        // Check for an optional initializer. If there is an '=' token, consume it and
        // parse the initializer expression.
        if let Some(token) = tokens.get(*pos)
            && token.token_type == "EQUAL"
        {
            *pos += 1; // consume '='
            initializer = Some(parse_expression(tokens, pos)?);
        }

        // Handle ";"
        let semicolon_token = tokens
            .get(*pos)
            .ok_or_else(|| "Syntax error: expected ';' after variable declaration.".to_string())?;
        if semicolon_token.token_type != "SEMICOLON" {
            return Err(format!(
                "[line {}:{}] Syntax error: expected ';' after variable declaration, but found '{}'.",
                semicolon_token.line, semicolon_token.col, semicolon_token.lexeme
            ));
        }
        *pos += 1; // consume ';'

        // Build the AST node and return it.
        let span = Span::new(
            Location::new(token.line, token.col),
            Location::new(
                semicolon_token.line,
                semicolon_token.col + semicolon_token.lexeme.len(),
            ),
        );
        Ok(ASTree::Declaration(Declaration::VarDecl(
            VarDecl::new(name, initializer, span),
            scope_depth,
        )))
    } else {
        // No match => go to the next rule
        parse_statement(tokens, pos, scope_depth)
    }
}

/// Parse a declaration, which can be a variable declaration or a statement (or lower).
///
/// `declaration → varDecl | statement`
fn parse_declaration(
    tokens: &[Token],
    pos: &mut usize,
    scope_depth: usize,
) -> Result<ASTree, String> {
    parse_var_decl(tokens, pos, scope_depth)
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
            let span = Span::new(
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
            let tokens_without_eof = if let Some(last_token) = tokens.last() {
                if last_token.token_type == "EOF" {
                    &tokens[..tokens.len() - 1]
                } else {
                    tokens
                }
            } else {
                tokens
            };
            let mut expr_pos = 0;
            let expr = parse_expression(tokens_without_eof, &mut expr_pos)?;
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
