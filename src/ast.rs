use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Namespace(String, Span),
    Use(String, Span),
    Require(String, Span),
    Import {
        names: Vec<String>,
        path: String,
        span: Span,
    },
    TypeAlias(TypeAliasDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
    Class(ClassDecl),
    Function(FnDecl),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub extends: Vec<String>,
    pub methods: Vec<MethodSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub backing_type: Option<String>, // "string" | "int"
    pub cases: Vec<EnumCase>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<ClassField>,
    pub constructor: Option<ConstructorDecl>,
    pub methods: Vec<MethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub visibility: Visibility,
    pub name: String, // without '$'
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub visibility: Visibility,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub visibility: Visibility,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String, // without '$'
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String, Vec<TypeExpr>, Span),
    Array(Box<TypeExpr>, Span),
    Union(Vec<TypeExpr>, Span),
    Optional(Box<TypeExpr>, Span),
    Shape(Vec<ShapeField>, Span),
    Function {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named(_, _, s)
            | TypeExpr::Array(_, s)
            | TypeExpr::Union(_, s)
            | TypeExpr::Optional(_, s)
            | TypeExpr::Shape(_, s)
            | TypeExpr::Function { span: s, .. } => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeField {
    pub name: String,
    pub optional: bool,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        name: String, // without '$'
        ty: Option<TypeExpr>,
        init: Expr,
        span: Span,
    },
    ConstDecl {
        name: String,
        ty: Option<TypeExpr>,
        init: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Echo {
        value: Expr,
        span: Span,
    },
    Global {
        vars: Vec<String>,
        span: Span,
    },
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    ForEach {
        iterable: Expr,
        key_var: Option<String>,
        val_var: String,
        body: Vec<Stmt>,
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64, Span),
    Float(f64, Span),
    String(String, Span),
    Bool(bool, Span),
    Null(Span),
    Var(String, Span),   // $x
    Ident(String, Span), // function/class/const name
    ArrayLit(Vec<Expr>, Span),
    AssocArrayLit(Vec<(String, Expr)>, Span), // ['a' => 1, 'b' => 2]
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Cast {
        target_ty: String, // (int), (float), (string), (bool)
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    InstanceOf {
        expr: Box<Expr>,
        class_name: String,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeExpr>,
        args: Vec<Expr>,
        span: Span,
    },
    New {
        class_name: String,
        type_args: Vec<TypeExpr>,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        property: String,
        span: Span,
    },
    StaticAccess {
        class_name: String,
        member: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    ArrowFn {
        params: Vec<Param>,
        return_type: Option<TypeExpr>,
        body: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat, // .
    EqEqEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    NullCoal,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::String(_, s)
            | Expr::Bool(_, s)
            | Expr::Null(s)
            | Expr::Var(_, s)
            | Expr::Ident(_, s)
            | Expr::ArrayLit(_, s)
            | Expr::AssocArrayLit(_, s)
            | Expr::Unary { span: s, .. }
            | Expr::Cast { span: s, .. }
            | Expr::Binary { span: s, .. }
            | Expr::InstanceOf { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::New { span: s, .. }
            | Expr::Member { span: s, .. }
            | Expr::StaticAccess { span: s, .. }
            | Expr::Index { span: s, .. }
            | Expr::ArrowFn { span: s, .. } => *s,
        }
    }
}
