mod lexer;
mod parser;

use serde::{Deserialize, Serialize};
use std::fmt::{self, Write};

pub use parser::parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    fn through(self, end: Self) -> Self {
        Self {
            start: self.start,
            end: end.end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn at(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "^",
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }

    pub fn compares(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Literal(String),
    Name(String),
    Negative(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    List(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: Kind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: Kind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn literal(text: impl Into<String>, span: Span) -> Self {
        Self::new(Kind::Literal(text.into()), span)
    }

    pub fn binary(op: Op, a: Self, b: Self, span: Span) -> Self {
        Self::new(Kind::Binary(op, Box::new(a), Box::new(b)), span)
    }

    pub fn negated(a: Self, span: Span) -> Self {
        Self::new(Kind::Negative(Box::new(a)), span)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::Literal(s) | Kind::Name(s) => f.write_str(s),
            Kind::Negative(a) => write!(f, "(-{a})"),
            Kind::Binary(op, a, b) => write!(f, "({a} {} {b})", op.symbol()),
            Kind::Call(name, args) => {
                write!(f, "{name}(")?;
                separated(f, args)?;
                f.write_char(')')
            }
            Kind::List(items) => {
                f.write_char('[')?;
                separated(f, items)?;
                f.write_char(']')
            }
        }
    }
}

fn separated(f: &mut fmt::Formatter<'_>, items: &[Expr]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Expr),
    Definition {
        name: String,
        params: Option<Vec<String>>,
        body: Expr,
        span: Span,
    },
}

impl Statement {
    pub fn expression(self) -> Result<Expr, Diagnostic> {
        match self {
            Self::Expression(expr) => Ok(expr),
            Self::Definition { span, .. } => {
                Err(Diagnostic::at(span, "Expected an expression here"))
            }
        }
    }
}
