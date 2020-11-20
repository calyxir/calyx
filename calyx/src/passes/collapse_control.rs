use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

#[derive(Default)]
/// Collapses and de-nests control constructs.
pub struct CollapseControl {}

impl Named for CollapseControl {
    fn name() -> &'static str {
        "collapse-control"
    }

    fn description() -> &'static str {
        "Collapse nested seq and par."
    }
}

impl Visitor<()> for CollapseControl {
    /// Collapse seq { seq { A }; B } into seq { A; B }.
    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _data: (),
        _comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        if s.stmts.is_empty() {
            return Ok(Action::change_default(ir::Control::empty()));
        }
        let mut seqs: Vec<ir::Control> = vec![];
        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Seq(mut data) => {
                    seqs.append(&mut data.stmts);
                }
                _ => seqs.push(con),
            }
        }
        Ok(Action::change_default(ir::Control::seq(seqs)))
    }

    /// Collapse par { par { A }; B } into par { A; B }.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _data: (),
        _comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        if s.stmts.is_empty() {
            return Ok(Action::change_default(ir::Control::empty()));
        }
        let mut pars: Vec<ir::Control> = vec![];
        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Par(mut data) => {
                    pars.append(&mut data.stmts);
                }
                _ => pars.push(con),
            }
        }
        Ok(Action::change_default(ir::Control::par(pars)))
    }
}
