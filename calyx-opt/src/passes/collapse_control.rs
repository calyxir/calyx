use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, GetAttributes, LibrarySignatures};

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
///
/// 3. Collapses nested `static seq` in the same way as 1
/// 4. Collapses nested `static par` in the same way as 2
/// 5. Collapses `static repeat`:
/// Collapse
/// ```
/// static repeat 0 { ** body ** }
/// ```
/// into empty control
/// and
/// ```
/// static repeat 1 {** body **}
/// ```
/// into
/// ```
/// ** body **
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
            if con.has_attribute(ir::BoolAttr::NewFSM) {
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

    /// Collapse static par {static par {A; B;}} into static par {A; B; }
    fn finish_static_par(
        &mut self,
        s: &mut ir::StaticPar,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.stmts.is_empty() {
            return Ok(Action::static_change(ir::StaticControl::empty()));
        }
        if s.stmts.len() == 1 {
            // Want to preserve @one_hot attribute.
            let mut replacement_ctrl = s.stmts.pop().unwrap();
            let attrs = std::mem::take(&mut s.attributes);
            replacement_ctrl
                .get_mut_attributes()
                .copy_from(attrs, vec![ir::BoolAttr::OneHot]);

            return Ok(Action::static_change(replacement_ctrl));
        }
        let mut pars: Vec<ir::StaticControl> = vec![];
        for con in s.stmts.drain(..) {
            match con {
                ir::StaticControl::Par(mut data) => {
                    pars.append(&mut data.stmts);
                }
                _ => pars.push(con),
            }
        }
        s.stmts = pars;
        Ok(Action::Continue)
    }

    ///Collase static seq {static seq {A; B; }} into static seq {A; B;}
    fn finish_static_seq(
        &mut self,
        s: &mut ir::StaticSeq,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.stmts.is_empty() {
            return Ok(Action::static_change(ir::StaticControl::empty()));
        }
        if s.stmts.len() == 1 {
            // Want to preserve @one_hot attribute.
            let mut replacement_ctrl = s.stmts.pop().unwrap();
            let attrs = std::mem::take(&mut s.attributes);
            replacement_ctrl
                .get_mut_attributes()
                .copy_from(attrs, vec![ir::BoolAttr::OneHot]);
            return Ok(Action::static_change(replacement_ctrl));
        }
        let mut seqs: Vec<ir::StaticControl> = vec![];
        for con in s.stmts.drain(..) {
            match con {
                ir::StaticControl::Seq(mut data) => {
                    seqs.append(&mut data.stmts);
                }
                _ => seqs.push(con),
            }
        }
        s.stmts = seqs;
        Ok(Action::Continue)
    }

    /// Collapse
    /// ```
    /// static repeat 0 { ** body ** }
    /// ```
    /// into empty control
    /// and
    /// ```
    /// static repeat 1 {** body **}
    /// into
    /// ```
    /// ** body **
    /// ```
    fn finish_static_repeat(
        &mut self,
        s: &mut ir::StaticRepeat,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.num_repeats == 0 {
            return Ok(Action::static_change(ir::StaticControl::empty()));
        }
        if s.num_repeats == 1 {
            return Ok(Action::static_change(s.body.take_static_control()));
        }
        Ok(Action::Continue)
    }

    fn finish_repeat(
        &mut self,
        s: &mut ir::Repeat,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if s.num_repeats == 0 {
            return Ok(Action::change(ir::Control::empty()));
        }
        if s.num_repeats == 1 {
            return Ok(Action::change(s.body.take_control()));
        }
        Ok(Action::Continue)
    }
}
