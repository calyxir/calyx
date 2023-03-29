use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;

#[derive(Default)]
/// Compiles Static Islands
pub struct CompileStatic;

impl Named for CompileStatic {
    fn name() -> &'static str {
        "compile-static"
    }

    fn description() -> &'static str {
        "Compiles Static  Islands"
    }
}

impl Visitor for CompileStatic {
    fn static_enable(
        &mut self,
        s: &mut ir::StaticEnable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        Ok(Action::Continue)
    }
}
