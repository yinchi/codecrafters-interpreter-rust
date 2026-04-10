//! Abstract Syntax Tree (AST) definitions for the Lox interpreter.

use super::span::Span;

pub mod impls;

/// A Lox program (top-level AST node)
///
/// program        → declaration* EOF ;
pub struct Program {
    pub declarations: Vec<Declaration>,
    #[allow(unused)]
    pub span: Span,
}

/// A declaration in the AST
///
/// declaration     → varDecl | statement ;
pub enum Declaration {
    VarDecl(VarDecl, usize), // usize is the indent depth (used for parser output indentation)
    Statement(Statement, usize), // usize is the indent depth (used for parser output indentation)
}

/// A variable declaration in the AST
///
/// varDecl         → "var" IDENTIFIER ( "=" expression )? ";" ;
pub struct VarDecl {
    pub name: String,
    pub initializer: Option<Expression>,
    #[allow(unused)]
    pub span: Span,
}

/// A statement in the AST, which can be an expression statement, a print statement, a while
/// statement, or a block.
///
/// Note that we don't have a separate `for` statement in the AST, since we can desugar `for`
/// loops into `while` loops in the parser.
///
/// `statement      → exprStmt | ifStmt | printStmt | whileStmt | forStmt | block ;`
/// `block          → "{" declaration* "}" ;`
pub enum Statement {
    ExprStmt(ExprStmt),
    IfStmt(IfStmt),
    PrintStmt(PrintStmt),
    WhileStmt(WhileStmt),
    Block(Vec<Declaration>),
}

/// An expression statement in the AST.
///
/// `exprStmt       → expression ";" ;`
pub struct ExprStmt {
    pub expr: Expression,
    pub span: Span,
}

/// An if statement in the AST.
///
/// `ifStmt         → "if" "(" expression ")" statement ( "else" statement )? ;`
pub struct IfStmt {
    pub condition: Expression,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
    #[allow(unused)]
    pub span: Span,
    pub indent_depth: usize,
}

/// A print statement in the AST.
///
/// `printStmt      → "print" exprStmt ;`
pub struct PrintStmt {
    pub expr_stmt: ExprStmt,
    #[allow(unused)]
    pub span: Span,
}

/// A while statement in the AST.
///
/// `whileStmt      → "while" "(" expression ")" statement ;`
pub struct WhileStmt {
    pub condition: Expression,
    pub body: Box<Statement>,
    #[allow(unused)]
    pub span: Span,
    pub indent_depth: usize,
}

/** An expression in the AST.

From the [Lox grammar, Sec. 6.1](https://craftinginterpreters.com/parsing-expressions.html),
we have the following rules for expressions:

```
expression     → assignment ;
assignment     → IDENTIFIER "=" assignment
               | logic_or ;
logic_or       → logic_and ( "or" logic_and )* ;
logic_and      → equality ( "and" equality )* ;
equality       → comparison ( ( "!=" | "==" ) comparison )* ;
comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
term           → factor ( ( "-" | "+" ) factor )* ;
factor         → unary ( ( "/" | "*" ) unary )* ;
unary          → ( "!" | "-" ) unary
               | primary ;
primary        → NUMBER | STRING | "true" | "false" | "nil"
               | "(" expression ")" ;
```

Note, for example, that "comparison" really means any expression at the comparison level or lower,
i.e. a primary expression will also match the unary, ..., and equality rules.
*/
pub enum ExpressionEnum {
    /*
    Note that we always box Expression instead of defining Unary, Factor, etc. enums,
    with precedence rules handled by the parse functions.
    */
    Primary(Primary),
    Unary(Operator, Box<Expression>),
    Factor(Operator, Box<Expression>, Box<Expression>),
    Term(Operator, Box<Expression>, Box<Expression>),
    Comparison(Operator, Box<Expression>, Box<Expression>),
    Equality(Operator, Box<Expression>, Box<Expression>),
    Logical(Operator, Box<Expression>, Box<Expression>),
    Assignment(Primary, Box<Expression>),
    // where Primary is the Identifier on the LHS
}

/// An expression in the AST along with its source code span, for error reporting purposes.
pub struct Expression {
    pub expr: ExpressionEnum,
    pub span: Span,
}

/// A primary expression in the AST along with its source code span, for error reporting purposes.
pub struct Primary {
    pub p: PrimaryEnum,
    pub span: Span,
}

pub enum PrimaryEnum {
    Literal(Literal),
    Grouping(Box<Expression>),
    Identifier(String),
}

/// A literal value in the AST, which can be a number, string, boolean, or nil.
#[derive(PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    True,
    False,
    Nil,
}

pub struct Operator {
    pub op: String,
    pub span: Span,
}
