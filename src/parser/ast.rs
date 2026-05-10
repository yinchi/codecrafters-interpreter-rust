//! Abstract Syntax Tree (AST) definitions for the Lox interpreter.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::span::{Location, Span};
use crate::environment::EnvRc;

pub mod impls;

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
/// declaration     → classDecl | funDecl | varDecl | statement ;
pub enum Declaration {
    ClassDecl(ClassDecl),
    FunDecl(FunDecl),
    VarDecl(VarDecl),
    Statement(Statement),
}

/// A class declaration in the AST
///
/// classDecl      → "class" IDENTIFIER ( "<" IDENTIFIER )? "{" function* "}" ;
///
/// Note that class functions are defined without the `fun` keyword, but we still represent
/// them as `FunDecl`s in the AST for simplicity.
#[derive(Clone, derive_new::new)]
pub struct ClassDecl {
    pub name: String,
    // To avoid implementing a bunch of `Clone` logic, we store the identifier's string and ID
    // directly here instead of storing an Primary::Identifier variant.
    pub superclass: Option<(String, usize)>,
    pub methods: Vec<FunDecl>,
    #[allow(unused)]
    pub span: Span,
}

/// A function declaration in the AST
///
/// funDecl         → "fun" IDENTIFIER "(" parameters? ")" block ;
#[derive(Clone, derive_new::new)]
pub struct FunDecl {
    pub name: String,
    /// Each element is `(parameter_name, source_location)`.
    pub params: Vec<(String, Location)>,
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
    /// Source location of the variable name token (used for error messages).
    pub name_loc: Location,
    pub initializer: Option<Expression>,
    #[allow(unused)]
    pub span: Span,
}

/// A statement in the AST. Note that we don't have a separate `for` statement in the AST,
/// since we can desugar `for` loops into `while` loops in the parser.
///
/// `statement      → exprStmt | ifStmt | printStmt | returnStmt | whileStmt | forStmt | block ;`
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
}

/** An expression in the AST.

```
expression     → assignment ;
assignment     → ( call "." )? IDENTIFIER "=" assignment
               | logic_or ;
logic_or       → logic_and ( "or" logic_and )* ;
logic_and      → equality ( "and" equality )* ;
equality       → comparison ( ( "!=" | "==" ) comparison )* ;
comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
term           → factor ( ( "-" | "+" ) factor )* ;
factor         → unary ( ( "/" | "*" ) unary )* ;
unary          → ( "!" | "-" ) unary
               | call ;
call           → primary ( "(" arguments? ")" | "." IDENTIFIER )* ;
primary        → NUMBER | STRING | "true" | "false" | "nil"
               | "(" expression ")" | IDENTIFIER | "this" | "super" "." IDENTIFIER ;
```

Note, for example, that "comparison" really means any expression at the comparison level or lower,
i.e. a primary expression will also match the unary, ..., and equality rules.
*/
pub enum ExpressionEnum {
    /*
    Note that we always box Expression instead of defining Unary, Factor, etc. enums,
    with precedence rules and subtype checking handled by the parse functions.
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
    Get(Box<Expression>, String),         // Object, Property name
    Set(Box<Expression>, String, Box<Expression>), // Object, Property name, Value
}

/// A list of arguments in a function call, along with the source code span for error reporting
/// purposes.
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
    Identifier(String, usize), // usize is the identifier ID (index of the token in the tokenize() output)
    This(usize), // usize is the identifier ID (index of the token in the tokenize() output)
    SuperDot(String, usize), // method name for the `super.<method>`, identifier ID of the "super" token
}

/// A literal value in the AST, which can be a number, string, boolean, or nil.
#[derive(Clone)]
pub enum Literal {
    Number(f64),
    String(String),
    Class(Rc<LoxClass>),
    Instance(InstanceRc),
    UserCallable(UserCallable),
    NativeCallable(NativeCallable),
    True,
    False,
    Nil,
}

/// A user-defined class value.
#[derive(Clone, derive_new::new)]
pub struct LoxClass {
    pub name: String,
    pub superclass: Option<Rc<LoxClass>>,
    pub methods: HashMap<String, UserCallable>,
    #[new(default)]
    id: Rc<()>,
}

pub type InstanceRc = Rc<RefCell<LoxInstance>>;

/// A user-defined class instance value.
#[derive(Clone, derive_new::new)]
pub struct LoxInstance {
    pub class: Rc<LoxClass>,
    pub fields: HashMap<String, Literal>,
}

/// A user-defined callable value in the AST.
#[derive(Clone, derive_new::new)]
pub struct UserCallable {
    /// The function declaration for this callable.
    /// We use an Rc here to allow multiple copies of the same function declaration,
    /// e.g. to allow patterns like `fun foo() {} print foo == foo;` to work as expected.
    pub decl: Rc<FunDecl>,

    /// The environment captured at the function's definition site (its closure).
    pub closure: EnvRc,

    /// Whether this function is an initializer for a class (default: false).
    #[new(default)]
    pub is_initializer: bool,
}

/// A native (built-in) callable value.
#[derive(Clone)]
pub struct NativeCallable {
    pub name: &'static str,
    pub arity: usize,
    pub func: fn(&[Literal]) -> Result<Literal, String>,
}

pub struct Operator {
    pub op: String,
    pub span: Span,
}
