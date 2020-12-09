use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Component, Control};
use crate::{build_assignments, structure};

#[derive(Default)]
pub struct CompileInvoke;

impl Named for CompileInvoke {
    fn name() -> &'static str {
        "compile-empty"
    }

    fn description() -> &'static str {
        "Rewrites empty control to invocation to empty group"
    }
}

impl Visitor for CompileInvoke {
}
