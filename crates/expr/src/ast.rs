#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Var {
        name: String,
        init: Expr,
    },
    Fn {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        yes: Box<Stmt>,
        no: Option<Box<Stmt>>,
        if_line: u32,
        else_line: Option<u32>,
    },
    Return(Option<Expr>),
    Block(Vec<Stmt>),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    Array(Vec<Expr>),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Assign {
        name: String,
        expr: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        yes: Box<Expr>,
        no: Box<Expr>,
    },
    Member {
        object: Box<Expr>,
        name: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Plus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    EqStrict,
    NotEqStrict,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}
