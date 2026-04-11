//! Abstract Syntax Tree (AST) definitions for the Lox interpreter.

use std::cell::RefCell;
use std::rc::Rc;

use super::span::Span;
use crate::environment::Environment;

pub mod impls;

type EnvRc = Rc<RefCell<Environment>>;

/// A Lox program (top-level AST node)
///
/// program        → declaration* EOF ;
#[derive(derive_new::new)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    #[allow(unused)]
    pub span: Span,
}

/// A declaration in the AST
///
/// declaration     → funDecl | varDecl | statement ;
pub enum Declaration {
    FunDecl(FunDecl, usize), // usize is the indent depth (used for parser output indentation)
    VarDecl(VarDecl, usize), // usize is the indent depth (used for parser output indentation)
    Statement(Statement, usize), // usize is the indent depth (used for parser output indentation)
}

/// A function declaration in the AST
///
/// funDecl         → "fun" IDENTIFIER "(" parameters? ")" block ;
#[derive(derive_new::new)]
pub struct FunDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Rc<Statement>, // the block statement representing the function body
    #[allow(unused)]
    pub span: Span,
}

/// A variable declaration in the AST
///
/// varDecl         → "var" IDENTIFIER ( "=" expression )? ";" ;
#[derive(derive_new::new)]
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
    ReturnStmt(ReturnStmt),
    WhileStmt(WhileStmt),
    Block(Vec<Declaration>),
}

/// An expression statement in the AST.
///
/// `exprStmt       → expression ";" ;`
#[derive(derive_new::new)]
pub struct ExprStmt {
    pub expr: Expression,
    pub span: Span,
}

/// An if statement in the AST.
///
/// `ifStmt         → "if" "(" expression ")" statement ( "else" statement )? ;`
#[derive(derive_new::new)]
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
#[derive(derive_new::new)]
pub struct PrintStmt {
    pub expr_stmt: ExprStmt,
    #[allow(unused)]
    pub span: Span,
}

/// A return statement in the AST.
///
/// `returnStmt     → "return" expression? ";" ;`
pub struct ReturnStmt {
    pub value: Option<Expression>, // Returns nil if no expression is provided.
    #[allow(unused)]
    pub span: Span,
}

/// A while statement in the AST.
///
/// `whileStmt      → "while" "(" expression ")" statement ;`
#[derive(derive_new::new)]
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
    Assignment(Primary, Box<Expression>), // where Primary is the Identifier on the LHS
    Call(Box<Expression>, Arguments),     // Callee, Arguments
}

/// A list of arguments in a function call, along with the source code span for error reporting purposes.
pub struct Arguments {
    pub args: Vec<Expression>,
    pub span: Span,
}

/// An expression in the AST along with its source code span, for error reporting purposes.
#[derive(derive_new::new)]
pub struct Expression {
    pub expr: ExpressionEnum,
    pub span: Span,
}

/// A primary expression in the AST along with its source code span, for error reporting purposes.
#[derive(derive_new::new)]
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
    Callable(Callable),
    True,
    False,
    Nil,
}

/// A user-defined callable value in the AST.  Note that built-in functions are
/// not represented as `Callable`s, since they don't need to be defined before being called.
#[derive(Clone, derive_new::new)]
pub struct Callable {
    pub decl: FunDecl,
    /// The environment captured at the function's definition site (its closure).
    pub closure: EnvRc,
}

impl PartialEq for Callable {
    fn eq(&self, other: &Self) -> bool {
        // Are self and other the same object in memory?
        std::ptr::eq(self, other)
    }
}

impl Clone for FunDecl {
    fn clone(&self) -> Self {
        FunDecl {
            name: self.name.clone(),
            params: self.params.clone(),
            body: Rc::clone(&self.body),
            span: self.span.clone(),
        }
    }
}

pub struct Operator {
    pub op: String,
    pub span: Span,
}
