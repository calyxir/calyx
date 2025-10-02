//! The AST types used to represent plan files and ways to traverse them

use camino::Utf8PathBuf;

use super::span::Span;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::ControlFlow};

/// The type of a lexer token.
#[derive(Clone, Debug)]
pub enum TokenKind {
    /// An identifier for a function or variable.
    Id(String),
    /// The assignment operator: `=`.
    Assign,
    OpenParen,
    CloseParen,
    Semicolon,
    Comma,
    /* TODO: add EOF kind for use in error handling */
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Id(id) => write!(f, "{id}"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::OpenParen => write!(f, "("),
            TokenKind::CloseParen => write!(f, ")"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Comma => write!(f, ","),
        }
    }
}

/// A lexer token.
#[derive(Clone, Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub span: Span<'a>,
}

/// The type returned by a visitor function.
pub trait VisitorResult {
    /// Visitors may find and return data throughout their run using `from_residual`. This is the
    /// type of that data. It is common this is set to `()`.
    type Residual;

    /// Returns a result build from nothing.
    fn output() -> Self;

    /// Returns a result built from a `Residual`.
    fn from_residual(r: Self::Residual) -> Self;

    /// Returns signal for how the visitor should continue traversing the AST.
    ///
    /// `ControlFlow::Continue(())` signals the visitor should continue, traversing the node's
    /// children. `ControlFlow::Break(r)` signals the visitor not traverse a node's children and
    /// instead to immediately return a `VisitorResult` built from `Residual` `r`.
    fn branch(self) -> ControlFlow<Self::Residual>;
}

/// It's very common to use a `ControlFlow` as a `VisitorResult` so the implementation is provided
/// here.
impl<T> VisitorResult for ControlFlow<T> {
    type Residual = T;

    fn output() -> Self {
        ControlFlow::Continue(())
    }

    fn from_residual(r: Self::Residual) -> Self {
        ControlFlow::Break(r)
    }

    fn branch(self) -> ControlFlow<Self::Residual> {
        self
    }
}

macro_rules! try_visit {
    ($e:expr) => {
        match $crate::plan_files::ast::VisitorResult::branch($e) {
            core::ops::ControlFlow::Continue(()) => (),
            core::ops::ControlFlow::Break(r) => {
                return $crate::plan_files::ast::VisitorResult::from_residual(
                    r,
                );
            }
        }
    };
}

/// Implemented by visitors of a flang AST.
pub trait Visitor {
    /// This is generally set to `std::ops::ControlFlow`. It is not done so here as a default
    /// because that is not yet a stable language feature in rust.
    type Result: VisitorResult;

    fn visit_op(&mut self, _f: &Op) -> Self::Result {
        Self::Result::output()
    }

    fn visit_assignment(&mut self, _a: &Assignment) -> Self::Result {
        Self::Result::output()
    }

    fn visit_assignment_list(&mut self, _a: &AssignmentList) -> Self::Result {
        Self::Result::output()
    }
}

pub trait Visitable<V: Visitor> {
    fn visit(&self, visitor: &mut V) -> V::Result;
}

pub(crate) type FunId = String;
pub(crate) type VarId = Utf8PathBuf;

/// A call to an op. For example, `calyx-to-verilog(infile)`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Op {
    pub name: FunId,
    pub args: Vec<VarId>,
}

impl<V: Visitor> Visitable<V> for Op {
    fn visit(&self, visitor: &mut V) -> V::Result {
        try_visit!(visitor.visit_op(self));
        V::Result::output()
    }
}

/// A list of variables being assigned to the result of an op. For example,
/// ```text
/// x, y = op1(in1, in2);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assignment {
    pub vars: Vec<VarId>,
    pub value: Op,
}

impl<V: Visitor> Visitable<V> for Assignment {
    fn visit(&self, visitor: &mut V) -> V::Result {
        try_visit!(self.value.visit(visitor));
        visitor.visit_assignment(self)
    }
}

/// A list of assignments making up a program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssignmentList {
    pub assigns: Vec<Assignment>,
}

impl<V: Visitor> Visitable<V> for AssignmentList {
    fn visit(&self, visitor: &mut V) -> V::Result {
        for assign in &self.assigns {
            try_visit!(assign.visit(visitor));
        }
        V::Result::output()
    }
}
