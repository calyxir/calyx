use crate::errors;
use crate::lang::component::Component;
use crate::lang::{
    ast, ast::Control, context::Context, structure::StructureGraph,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

/// Pass that collapses
/// ```
/// (seq (enable A B)
///      (enable C D))
///      ..)
/// ```
/// into
/// ```
/// (seq (enable A B C D)
///      ..)
/// ```
///
/// when the sub-graphs induced by (enable A B) and (enable C D) have to common
/// edges (i.e. cannot observe each other's computation).
///
/// For example, suppose that this were the structure graph of your component:
/// ```
/// ╭─╮    ╭─╮
/// │A│    │C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// In this case, the program
/// ```
/// (seq (enable A B) (enable C D))
/// ```
/// is equivalent to
/// ```
/// (seq (enable A B C D))
/// ```
/// because there are no edges between the sub-graph induced by `A` and `B`
/// and the sub-graph induced by `C` and `D`.
///
/// If instead this were your component graph:
/// ```
/// ╭─╮    ╭─╮
/// │A│───>│C│
/// ╰┬╯    ╰┬╯
///  │      │
///  │      │
///  v      v
/// ╭─╮    ╭─╮
/// │B│    │D│
/// ╰─╯    ╰─╯
/// ```
/// then you could not collapse the `seq`.

// XXX (zhijing): I think this pass need to be changed if we add `enable` CSP style components to futil semantics
#[derive(Default)]
pub struct AutomaticPar {}

impl Named for AutomaticPar {
    fn name() -> &'static str {
        "automatic-par"
    }

    fn description() -> &'static str {
        "automatically parallelizes some sequential enables"
    }
}

// Check if two given components have an edge between them.
fn has_conflicts(
    comps1: &[ast::Id],
    comps2: &[ast::Id],
    st: &StructureGraph,
) -> Result<bool, errors::Error> {
    Ok(
        // check if comps1 shows up in comps2 ever
        comps1.iter().any(|id| comps2.contains(id))
            || comps2
                .iter()
                .map(|en_comp| st.get_inst_index(en_comp))
                // If any of the get_inst_index failed, return the error.
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                // any is shortcircuiting. It returnrs on the first true.
                .any(|idx| {
                    // for every output port, check if any incoming/outgoing edges
                    // contain a component in `comps1`
                    st.graph[idx].out_ports().any(|port| {
                        st.connected_outgoing(idx, port.to_string())
                            .chain(st.connected_incoming(idx, port.to_string()))
                            .any(|(node_data, _)| {
                                comps1.contains(node_data.get_name())
                            })
                    })
                }),
    )
}

impl Visitor for AutomaticPar {
    // use finish_seq so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_seq(
        &mut self,
        seq: &ast::Seq,
        comp: &mut Component,
        _: &Context,
    ) -> VisResult {
        // get first enable statement and its index. this will act
        // as the accumulator for the statement that we are collapsing
        // things into
        let (start, mut cmp_acc) = match seq.stmts.iter().enumerate().find_map(
            |(i, stmt)| match stmt {
                Control::Enable { data } => Some((i, data.clone())),
                _ => None,
            },
        ) {
            Some((s, c)) => (s, c),
            None => return Ok(Action::Continue),
        };

        // vec of new control statements including all elements up to the first
        // enable that we find
        let mut new_stmts: Vec<ast::Control> = seq.stmts[..start].to_vec();

        // start interation from the start+1 because the start is in `cmp_stmt`.
        // This loop will keep trying to merge adjacent sequences as long as
        // there are no conflicts. When it detects a conflict, it attempts
        // to merge the sequence after the conflicting node.
        for stmt in seq.stmts[start + 1..].iter() {
            match stmt {
                Control::Enable { data: enables2 } => {
                    // for every component in the `enables2` we check if any
                    // incoming/outgoing edge has an endpoint in `cmp_acc`
                    if has_conflicts(
                        &cmp_acc.comps,
                        &enables2.comps,
                        &comp.structure,
                    )? {
                        new_stmts.push(Control::enable(cmp_acc.comps.clone()));
                        // there was a conflict, start, a new accumulator.
                        cmp_acc = enables2.clone();
                    } else {
                        // If there is no conflict, update the current
                        // component accumulator.
                        cmp_acc.comps.append(&mut enables2.comps.clone());
                    }
                }
                _ => {
                    // Push the currently collected component accumulator.
                    if !cmp_acc.comps.is_empty() {
                        new_stmts.push(Control::enable(cmp_acc.comps.clone()));
                    }
                    new_stmts.push(stmt.clone());
                    // Add a new cmp_acc.
                    cmp_acc = ast::Enable { comps: Vec::new() };
                }
            }
        }

        if !cmp_acc.comps.is_empty() {
            new_stmts.push(Control::enable(cmp_acc.comps));
        }

        Ok(Action::Change(Control::seq(new_stmts)))
    }
}
