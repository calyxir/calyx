use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::GetAttributes;

#[derive(Default)]
/// Transforms a par of seq blocks into a seq of par blocks. For the
/// transformation to occur, each seq block must have the same number of statements,
/// and the nth statement of each seq block in the par block
/// must all have the same number of clock cycles.
///
/// #Example
/// par {
///    seq { @static(M) A0; @static(N) B0; @static(P) C0; }
///    seq { @static(M) A1; @static(N) B1; @static(P) C1; }
///    seq { @static(M) A2; @static(N) B2; @static(P) C2; }
///}
///
/// into
///
/// seq {
///     par{ @static(M) A0; @static(M) A1; @static(M) A2;}
///     par{ @static(N) B0; @static(N) B1; @static(N) B2;}
///     par{ @static(P) C0; @static(P) C1; @static(P) C2;}
/// }
///  
pub struct StaticParConv;

impl Named for StaticParConv {
    fn name() -> &'static str {
        "static-par-conv"
    }

    fn description() -> &'static str {
        "Transform `par` of `seq` to `seq` of `par` under correct conditions"
    }
}

// Given a Control statement, returns the length of stmts if the Control statement
//  is a Seq. Panics if Control statement is not a Seq.
fn len_if_seq(stmt: &ir::Control) -> usize {
    match stmt {
        ir::Control::Seq(seq) => seq.stmts.len(),
        _ => panic!("Not a sequence"),
    }
}

// Given a Control statement and an index n.
// If the Control statement is not a Seq, then panics.
// Othrwise, returns Some(cycles), where cycles is the value of the "static"
// attribute of the nth stmt in the Seq. Returns None if no such value is available.
fn cycles_if_seq(s: &ir::Control, index: usize) -> Option<&u64> {
    match s {
        ir::Control::Seq(seq) => seq.stmts.get(index).and_then(|stmt| {
            stmt.get_attributes().and_then(|atts| atts.get("static"))
        }),
        _ => panic!("Not a sequence"),
    }
}

impl Visitor for StaticParConv {
    fn finish_par(
        &mut self,
        s: &mut ir::Par,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        //if the par block is empty we can stop
        if (*s).stmts.is_empty() {
            return Ok(Action::Continue);
        }

        //make sure only have seq stmts
        if !(*s)
            .stmts
            .iter()
            .all(|x| matches!(x, ir::Control::Seq(_seq)))
        {
            return Ok(Action::Continue);
        }

        //make sure each seq has the same number of stmts
        let lens_vec =
            (*s).stmts.iter().map(len_if_seq).collect::<Vec<usize>>();
        let min_seq_len = *lens_vec
            .iter()
            .min()
            .unwrap_or_else(|| panic!("empty par block"));
        let max_seq_len = *lens_vec
            .iter()
            .max()
            .unwrap_or_else(|| panic!("empty par block"));
        if min_seq_len != max_seq_len {
            return Ok(Action::Continue);
        }
        let seq_len = min_seq_len;

        //make sure nth statement in each seq takes same number of cycle
        for n in 0..seq_len {
            let cycles_vec = (*s)
                .stmts
                .iter()
                .map(|stmt| cycles_if_seq(stmt, n))
                .collect::<Vec<Option<&u64>>>();
            if cycles_vec.is_empty() {
                panic!("empty par block");
            }
            let fst = cycles_vec[0];
            if fst == None {
                return Ok(Action::Continue);
            }
            if !cycles_vec.into_iter().all(|xth| xth == fst) {
                return Ok(Action::Continue);
            }
        }

        //It complains when I try to use the vec![] syntax. It says:
        //the trait `Clone` is not implemented for `ir::control::Control`
        //note: required by a bound in `from_elem`
        //So I am doing it this way instead.
        let mut new_seq_stmts = Vec::new();
        for _n in 0..seq_len {
            new_seq_stmts.push(Vec::new());
        }

        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Seq(mut seq) => {
                    for (counter, stmt) in seq.stmts.drain(..).enumerate() {
                        new_seq_stmts[counter].push(stmt);
                    }
                }
                _ => panic!("Encountered non sequences"),
            }
        }

        let par_vec = new_seq_stmts.into_iter().map(ir::Control::par).collect();

        Ok(Action::Change(Box::new(ir::Control::seq(par_vec))))
    }
}
