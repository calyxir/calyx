use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};

/// Pass to do something
#[derive(Default)]
pub struct MergeStaticPar;

impl Named for MergeStaticPar {
    fn name() -> &'static str {
        "merge-static-par"
    }

    fn description() -> &'static str {
        "merge static pars when they have the same static time"
    }
}

impl Visitor for MergeStaticPar {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        for stmt in &s.stmts {
            let mut err = std::io::stderr();
            ir::Printer::write_control(stmt, 0, &mut err)?;
        }
        Ok(Action::Continue)
    }
}
