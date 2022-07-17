use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use std::cmp::Ordering;

#[derive(Default)]
/// Transforms a par of seq blocks into a seq of par blocks. It will sometimes only
/// apply this transformation to a subset of seq blocks in the par block.
/// Regardless, this transformation should never increase the number of cycles the par
/// block takes to execute.
///
/// #Example
///
/// ```
/// par {
///    seq { @static(M) A0; @static(N) B0; @static(P) C0; }
///    seq { @static(M) A1; @static(N) B1; @static(P) C1; }
///    seq { @static(M) A2; @static(N) B2; @static(P) C2; }
///}
/// ```
///
/// into
///
/// ```
/// @static(M + N + P) seq {
///     @static(M) par{ @static(M) A0; @static(M) A1; @static(M) A2;}
///     @static(N) par{ @static(N) B0; @static(N) B1; @static(N) B2;}
///     @static(P) par{ @static(P) C0; @static(P) C1; @static(P) C2;}
/// }
/// ```

pub struct StaticParConv;

impl Named for StaticParConv {
    fn name() -> &'static str {
        "static-par-conv"
    }

    fn description() -> &'static str {
        "Transform `par` of `seq` to `seq` of `par` under correct conditions"
    }
}

// Takes two seqs, longer and shorter. longer should be at least as long as
// shorter. Returns Some(vec) if there exists arrangement of shorter and longer
// such that each statement in shorter can be paired with a statement in longer,
// such that executing these pairs as a seq of pars will respect dependencies and
// the number of cycles it takes will remain the same. If no such vec exists, returns None.
// The vec it returns contains which indices of longer each element in shorter
// should be paired with. So, if we represent the vecs by the static attribute of their
// statements, and longer = [4,1,5] and shorter = [4,5] then vec would be
// [0,2], since the 4 in shorter can be paired with the 4 in longer (@ index 0)
// and the 5 in shorter can be paired with the 5 in longer (@ index 2).
// A consequence of this is that vec should always be the same length as shorter.
fn is_compatible(longer: &ir::Seq, shorter: &ir::Seq) -> Option<Vec<usize>> {
    let mut long_iter = (*longer).stmts.iter();
    let mut short_iter = (*shorter).stmts.iter();

    let mut long_val = long_iter.next();
    let mut short_val = short_iter.next();

    let mut index_counter = Vec::new();
    let mut counter = 0;

    while let (Some(c1), Some(c2)) = (long_val, short_val) {
        match (c1.get_attribute("static"), c2.get_attribute("static")) {
            (Some(x1), Some(x2)) => {
                if x2 <= x1 {
                    short_val = short_iter.next();
                    index_counter.push(counter);
                }
                long_val = long_iter.next();
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

// Returns a default Attribute with "static" set to v
fn attribute_with_static(v: u64) -> ir::Attributes {
    let mut atts = ir::Attributes::default();
    atts.insert("static", v);
    atts
}

// Returns the Some(sum), where sum is the sum of the static attribute for each
// stmt in seq. Returns None if there is at least one statement in seq
// that does not have a static attribute
fn get_static_sum(seq: &ir::Seq) -> Option<u64> {
    let static_vals = seq
        .stmts
        .iter()
        .map(|c| c.get_attribute("static"))
        .collect::<Option<Vec<&u64>>>()?;

    Some(static_vals.into_iter().sum())
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

        let (mut to_be_partitioned, mut has_been_partitioned) =
            (Vec::new(), Vec::new());
        for con in s.stmts.drain(..) {
            match con {
                ir::Control::Seq(seq) => to_be_partitioned.push(seq),
                x => has_been_partitioned.push(x),
            }
        }

        //sort from longest seq to shortest
        to_be_partitioned.sort_by(|s1, s2| {
            let len1 = s1.stmts.len();
            let static1 = get_static_sum(s1);
            let len2 = s2.stmts.len();
            let static2 = get_static_sum(s2);
            match len2.cmp(&len1) {
                Ordering::Equal => static2.cmp(&static1),
                x => x,
            }
        });

        while !to_be_partitioned.is_empty() {
            let longest_seq = to_be_partitioned.remove(0);
            let max_seq_len = longest_seq.stmts.len();

            //if we just have a bunch of empty seqs
            if max_seq_len == 0 {
                break;
            }

            //the stmts in this vec of vecs will eventually be transformed into a
            //seq of pars. The usize is used to determine which par block
            //the respective statement will eventually be put in.
            let mut partition_group: Vec<Vec<(ir::Control, usize)>> = vec![];

            //looking for seqs that are compatible with longest_seq
            let mut i = 0;
            while i != to_be_partitioned.len() {
                if let Some(index_vec) =
                    is_compatible(&longest_seq, &to_be_partitioned[i])
                {
                    let seq = to_be_partitioned.remove(i);
                    if seq.stmts.len() != index_vec.len() {
                        unreachable!("seq should be same len as indices")
                    }
                    let indexed_stmts: Vec<(ir::Control, usize)> = seq
                        .stmts
                        .into_iter()
                        .zip(index_vec.into_iter())
                        .collect();
                    partition_group.push(indexed_stmts);
                } else {
                    i += 1;
                }
            }

            //if there are no seqs compatible w/ longest_seq, then
            //there is no point in making a seq of pars since we just
            //have a single seq (namely longest_seq)
            if partition_group.is_empty() {
                has_been_partitioned.push(ir::Control::Seq(longest_seq));
                continue;
            };

            //adding longest_seq to partition_group
            let longest_seq_indexed =
                longest_seq.stmts.into_iter().zip(0..max_seq_len).collect();
            partition_group.push(longest_seq_indexed);

            //now we start the process of creating a seq of pars
            let mut new_pars_stmts = Vec::new();
            for _n in 0..max_seq_len {
                new_pars_stmts.push(Vec::new());
            }

            for mut v in partition_group.drain(..) {
                for (stmt, index) in v.drain(..) {
                    new_pars_stmts[index].push(stmt);
                }
            }

            //getting the static attribute for each of the par blocks we will build
            let new_pars_static = new_pars_stmts
                .iter()
                .map(|stmts| {
                    stmts
                        .iter()
                        .map(|stmt| {
                            match stmt.get_attribute("static")
                            {
                                Some(&x1) => x1,
                                None => unreachable!("every statement in the new par blocks should have a static attribute"),
                            }
                        })
                        .max()
                })
                .collect::<Option<Vec<u64>>>().unwrap_or_else(|| unreachable!("none of the par blocks should be empty"));

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

            has_been_partitioned.push(new_seq);
        }

        //no need for a par block if the par block will only contain one control statement
        if has_been_partitioned.len() == 1 {
            return Ok(Action::Change(Box::new(
                has_been_partitioned.remove(0),
            )));
        }

        Ok(Action::Change(Box::new(ir::Control::par(
            has_been_partitioned,
        ))))
    }
}
