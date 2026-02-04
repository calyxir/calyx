//! The AST types used to represent plan files and ways to traverse them. This is primarily used
//! serialization and deserialization of the AST. A more efficient and ergonomic representation of
//! flang is in `Ir` which is manipulated by the internals of the program.

use camino::Utf8PathBuf;

use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;

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
        match $crate::flang::ast::VisitorResult::branch($e) {
            core::ops::ControlFlow::Continue(()) => (),
            core::ops::ControlFlow::Break(r) => {
                return $crate::flang::ast::VisitorResult::from_residual(r);
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

/// A list of assignments making up a program. This creates a straightforward AST used for
/// serialization and deserialization.
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

/// The assignment list making up the program combined with a header specifying which files are
/// inputs and outputs and which of those inputs and outputs should be written to/read from stdio.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prog {
    /// The input files to be read from stdin.
    pub stdins: Vec<Utf8PathBuf>,

    /// The input files to be written to stdout.
    pub stdouts: Vec<Utf8PathBuf>,

    /// The input files.
    pub inputs: Vec<Utf8PathBuf>,

    /// The output files.
    pub outputs: Vec<Utf8PathBuf>,

    /// The flang AST.
    pub ast: AssignmentList,
}
