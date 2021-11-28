use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, LibrarySignatures};

/// Inlines all sub-components marked with the `@inline` attribute.
/// Cannot inline components when they:
///   1. Are primitives
///   2. Are invoked structurally
///   3. Invoked using `invoke`-`with` statements
///
/// For each component that needs to be inlined, we need to:
///   1. Inline all cells defined by that instance.
///   2. Inline all groups defined by that instance.
///   3. Inline the control program for every `invoke` statement referring to the
///      instance.
#[derive(Default)]
pub struct ComponentInliner;

impl Named for ComponentInliner {
    fn name() -> &'static str {
        "inline"
    }

    fn description() -> &'static str {
        "attempts to inline all components marked with @inline attribute"
    }
}

impl Visitor for ComponentInliner {
    fn start(
        &mut self,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        _s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        Ok(Action::Continue)
    }
}
