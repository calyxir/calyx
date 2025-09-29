//! The AST types used to represent plan files and ways to traverse them

use camino::Utf8PathBuf;

use super::span::Span;
use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;

#[derive(Clone, Debug)]
pub enum TokenKind {
    Id(String),
    Assign,
    OpenParen,
    CloseParen,
    Semicolon,
    Comma,
    /* TODO: add EOF kind for use in error handling */
}

#[derive(Clone, Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub span: Span<'a>,
}

pub trait VisitorResult {
    type Residual;
    fn output() -> Self;
    fn from_residual(r: Self::Residual) -> Self;
    fn branch(self) -> ControlFlow<Self::Residual>;
}

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

pub trait Visitor {
    /// This is generally set to `std::ops::ControlFlow`. It is not done so here because that isn't
    /// yet a stable language feature in rust.
    type Result: VisitorResult;

    fn visit_function(&mut self, _f: &Function) -> Self::Result {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: FunId,
    pub args: Vec<VarId>,
}

impl<V: Visitor> Visitable<V> for Function {
    fn visit(&self, visitor: &mut V) -> V::Result {
        try_visit!(visitor.visit_function(self));
        V::Result::output()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assignment {
    pub vars: Vec<VarId>,
    pub value: Function,
}

impl<V: Visitor> Visitable<V> for Assignment {
    fn visit(&self, visitor: &mut V) -> V::Result {
        try_visit!(self.value.visit(visitor));
        visitor.visit_assignment(self)
    }
}

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
