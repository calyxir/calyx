use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};

#[derive(Default)]
/// Collapses and de-nests control constructs.
///
/// Running this pass removes unnecessary FSM transitions and compilation
/// groups during the lowering phase. If a seq is marked with @new_fsm, then
/// we don't collapse it, since we need that fsm transition to transition
/// from our old fsm to our new one.
///
/// # Example
/// 1. Collapses nested `seq`:
/// ```
/// seq {
///     seq { A; B }
///     C;
/// }
/// ```
/// into
/// ```
/// seq { A; B C; }
/// ```
/// 2. Collapses nested `par`:
/// ```
/// par {
///     par { A; B }
///     C;
/// }
/// ```
/// into
/// ```
/// par { A; B C; }
/// ```
pub struct CollapseControl {}

impl Named for CollapseControl {
    fn name() -> &'static str {
        "collapse-control"
    }

    fn description() -> &'static str {
        "Collapse nested seq and par."
    }
}

impl Visitor for CollapseControl {
    /// Collapse seq { seq { A }; B } into seq { A; B }.
    fn finish_seq(
        &mut self,
        s: &mut ir::Seq,
        _comp: &mut ir::Component,
        _c: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.stmts.is_empty() {
            return Ok(Action::change(ir::Control::empty()));
        }
        if s.stmts.len() == 1 {
            return Ok(Action::change(s.stmts.pop().unwrap()));
        }
        let mut seqs: Vec<ir::Control> = vec![];
        for con in s.stmts.drain(..) {
            if con.has_attribute(ir::Attribute::NewFSM) {
                // if con has attribute new_fsm, then we do *not* want to collapse
                seqs.push(con)
            } else {
                match con {
                    ir::Control::Seq(mut data) => {
                        seqs.append(&mut data.stmts);
                    }
                    _ => seqs.push(con),
                }
            }
        }
        s.stmts = seqs;
        Ok(Action::Continue)
    }

    /// Collapse par { par { A }; B } into par { A; B }.
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _c: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.stmts.is_empty() {
            return Ok(Action::change(ir::Control::empty()));
        }
        if s.stmts.len() == 1 {
            return Ok(Action::change(s.stmts.pop().unwrap()));
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
        s.stmts = pars;
        Ok(Action::Continue)
    }
}
