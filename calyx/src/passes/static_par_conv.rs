use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::GetAttributes;

#[derive(Default)]
/// Transforms a par of seq blocks into a seq of par blocks. For the
/// transformation to occur, each seq block must have the same number of statements,
/// and the nth statement of each seq block in the par block
/// must all have the same number of clock cycles. (Subject to change)
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
/// @static(M + N + P) seq {
///     @static(M) par{ @static(M) A0; @static(M) A1; @static(M) A2;}
///     @static(N) par{ @static(N) B0; @static(N) B1; @static(N) B2;}
///     @static(P) par{ @static(P) C0; @static(P) C1; @static(P) C2;}
/// }

pub struct StaticParConv;

enum StaticAttr {
    NoAttr,
    NoStmt,
    StaticVal(u64),
}

impl Named for StaticParConv {
    fn name() -> &'static str {
        "static-par-conv"
    }

    fn description() -> &'static str {
        "Transform `par` of `seq` to `seq` of `par` under correct conditions"
    }
}

// Given a Control statement, returns the Some(length), where length
// is the length of stmts if the Control statement
// is a Seq. None if Control statement is not a Seq.
fn len_if_seq(stmt: &ir::Control) -> Option<usize> {
    match stmt {
        ir::Control::Seq(seq) => Some(seq.stmts.len()),
        _ => None,
    }
}

// Given a Control statement and an index n.
// If the Control statement is not a Seq, then unreachable!
// Otherwise, returns a StaticAttr. It returns NoStmt if there
// exists no nth statement in the Seq. It returns NoAttr if
// the nth statement exists, but does not have a "static" attribute.
// It returns StaticVal(static_val), where static_val is the
// value of the "static" attribute of the nth statement.
fn cycles_if_seq(s: &ir::Control, index: usize) -> StaticAttr {
    match s {
        ir::Control::Seq(seq) => {
            if let Some(stmt) = seq.stmts.get(index) {
                match stmt.get_attributes().and_then(|atts| atts.get("static"))
                {
                    None => StaticAttr::NoAttr,
                    Some(&static_val) => StaticAttr::StaticVal(static_val),
                }
            } else {
                StaticAttr::NoStmt
            }
        }
        _ => unreachable!("Not a sequence"),
    }
}

// returns a default Attribute with "static" set to v
fn attribute_with_static(v: u64) -> ir::Attributes {
    let mut atts = ir::Attributes::default();
    atts.insert("static", v);
    atts
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
        if s.stmts.is_empty() {
            return Ok(Action::Continue);
        }

        //make sure there are only seqs, and if so, get the length of the longest seq
        let lens_vec = s
            .stmts
            .iter()
            .map(len_if_seq)
            .collect::<Option<Vec<usize>>>();
        let max_seq_len = if let Some(lens) = lens_vec {
            lens.into_iter()
                .max()
                .unwrap_or_else(|| unreachable!("empty par block"))
        } else {
            return Ok(Action::Continue);
        };

        //vec to hold the @static vals for each par block in the seq we will create
        let mut new_pars_static = Vec::new();

        //make sure nth statement in each seq (if it exists) takes same number of cycles
        for n in 0..max_seq_len {
            let cycles_vec = s
                .stmts
                .iter()
                .map(|stmt| cycles_if_seq(stmt, n))
                .collect::<Vec<StaticAttr>>();
            // There is an uglier way to do this doing just 1 iteration
            let cycles_val = cycles_vec
                .iter()
                .find(|x| matches!(x, StaticAttr::StaticVal(_v)));
            match cycles_val {
                Some(&StaticAttr::StaticVal(v)) => {
                    if cycles_vec.into_iter().all(|static_attr| {
                        match static_attr {
                            StaticAttr::StaticVal(x) => x == v,
                            StaticAttr::NoStmt => true,
                            StaticAttr::NoAttr => false,
                        }
                    }) {
                        new_pars_static.push(v);
                    } else {
                        return Ok(Action::Continue);
                    }
                }
                _ => return Ok(Action::Continue),
            };
        }

        let mut new_pars_stmts = Vec::new();
        for _n in 0..max_seq_len {
            new_pars_stmts.push(Vec::new());
        }

        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Seq(mut seq) => {
                    for (counter, stmt) in seq.stmts.drain(..).enumerate() {
                        new_pars_stmts[counter].push(stmt);
                    }
                }
                _ => unreachable!("encountered non sequence"),
            }
        }

        //the @static attribute for the entire seq block I will create
        let new_seq_static: u64 = new_pars_static.iter().sum();

        let pars_vec = new_pars_stmts
            .into_iter()
            .zip(new_pars_static.into_iter())
            .map(|(s, v)| {
                ir::Control::Par(ir::Par {
                    stmts: s,
                    attributes: attribute_with_static(v),
                })
            })
            .collect();

        Ok(Action::Change(Box::new(ir::Control::Seq(ir::Seq {
            stmts: pars_vec,
            attributes: attribute_with_static(new_seq_static),
        }))))
    }
}
