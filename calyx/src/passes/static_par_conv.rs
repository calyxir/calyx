use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::GetAttributes;

#[derive(Default)]
/// Transforms a par of seq blocks into a seq of par blocks. It will sometimes only
/// apply this transformation to a subset of seq blocks in the par block.
/// This transformation should never increase the number of cycles the par
/// block takes to execute.
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

impl Named for StaticParConv {
    fn name() -> &'static str {
        "static-par-conv"
    }

    fn description() -> &'static str {
        "Transform `par` of `seq` to `seq` of `par` under correct conditions"
    }
}

// Takes in two seqs, longer and shorter. longer should be at least as long as
// shorter. Returns Some(vec) if there exists arrangement of shorter and longer
// such that each statement in shorter can be paired with a statement in longer,
// such that executing these pairs in as a seq of pars will respect dependencies and
// the number of cycles it takes will remaine the same (i.e., the same as longer)/
// The vec it returns the indices that each element of shorter should be paired
// with longer. So, if we represent the vecs by the static attribute of their
// statements, if longer = [4,1,5] and shorter [4,5] then vec would be
// [0,2], since the 4 in shorter lines up with the 4 in longer (@ index 0)
// and the 5 in shorter lines up with 5 in longer (@ index 2). A consequence of this is
// that vec should always be the same length as shorter.
fn is_compatible(longer: &ir::Seq, shorter: &ir::Seq) -> Option<Vec<usize>> {
    let mut long_iter = (*longer).stmts.iter();
    let mut short_iter = (*shorter).stmts.iter();

    let mut long_val = long_iter.next();
    let mut short_val = short_iter.next();

    let mut index_counter = Vec::new();
    let mut counter = 0;

    while let (Some(c1), Some(c2)) = (long_val, short_val) {
        match (
            c1.get_attributes().and_then(|atts| atts.get("static")),
            c2.get_attributes().and_then(|atts| atts.get("static")),
        ) {
            (Some(x1), Some(x2)) => {
                if x2 <= x1 {
                    long_val = long_iter.next();
                    short_val = short_iter.next();
                    index_counter.push(counter);
                } else {
                    long_val = long_iter.next();
                }
            }
            _ => return None,
        }
        counter += 1;
    }

    match short_val {
        None => Some(index_counter),
        Some(_) => None,
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

        // non_compatible holds everything that will NOT be turned into a
        // seq of pars
        let (mut seqs, mut non_compatible) = (Vec::new(), Vec::new());
        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Seq(seq) => seqs.push(seq),
                x => non_compatible.push(x),
            }
        }

        if seqs.is_empty() {
            return Ok(Action::Change(Box::new(ir::Control::par(
                non_compatible,
            ))));
        }

        //is there a more idiomatic way to take ownership of just the longest seq
        //but not the others?
        let max_seq_len = seqs
            .iter()
            .map(|seq| seq.stmts.len())
            .max()
            .unwrap_or_else(|| unreachable!("no seqs"));
        let longest_seq;
        if let Some(p) =
            seqs.iter().position(|seq| seq.stmts.len() == max_seq_len)
        {
            longest_seq = seqs.swap_remove(p);
        } else {
            unreachable!("no seq that has length max_seq_len")
        }

        let mut indexed_seqs = vec![];

        //organizing seqs into those that are compatible w/ longest_seq
        //and those that are not
        for seq in seqs.drain(..) {
            if let Some(index_vec) = is_compatible(&longest_seq, &seq) {
                indexed_seqs.push((seq, index_vec));
            } else {
                non_compatible.push(ir::Control::Seq(seq));
            }
        }

        if indexed_seqs.is_empty() {
            non_compatible.push(ir::Control::Seq(longest_seq));
            return Ok(Action::Change(Box::new(ir::Control::par(
                non_compatible,
            ))));
        }

        indexed_seqs.push((longest_seq, (0..max_seq_len).collect()));

        let mut new_pars_stmts = Vec::new();
        for _n in 0..max_seq_len {
            new_pars_stmts.push(Vec::new());
        }

        for (seq, indices) in indexed_seqs.drain(..) {
            let mut labeled_stmts: Vec<(ir::Control, usize)> =
                seq.stmts.into_iter().zip(indices.into_iter()).collect();
            for (stmt, index) in labeled_stmts.drain(..) {
                new_pars_stmts[index].push(stmt);
            }
        }

        let new_pars_static = match new_pars_stmts
            .iter()
            .map(|vec| {
                vec
                    .iter()
                    .map(|stmt| {
                        match stmt
                            .get_attributes()
                            .and_then(|atts| atts.get("static"))
                        {
                            Some(&x1) => x1,
                            None => unreachable!("every statement in the new par blocks should have a static attribute"),
                        }
                    })
                    .max()
            })
            .collect::<Option<Vec<u64>>>()
        {
            Some(vec) => vec,
            None => unreachable!("none of the par blocks should be empty"),
        };

        let new_seq_static = new_pars_static.iter().sum();

        let new_pars: Vec<ir::Control> = new_pars_stmts
            .into_iter()
            .zip(new_pars_static.into_iter())
            .map(|(stmts_vec, static_attr)| {
                ir::Control::Par(ir::Par {
                    stmts: stmts_vec,
                    attributes: attribute_with_static(static_attr),
                })
            })
            .collect();

        let new_seq = ir::Control::Seq(ir::Seq {
            stmts: new_pars,
            attributes: attribute_with_static(new_seq_static),
        });

        if non_compatible.is_empty() {
            return Ok(Action::Change(Box::new(new_seq)));
        }

        non_compatible.push(new_seq);
        Ok(Action::Change(Box::new(ir::Control::par(non_compatible))))
    }
}
