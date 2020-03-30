use crate::lang::component::Component;
use crate::lang::{ast, ast::Control, context::Context};
use crate::passes::visitor::{Action, VisResult, Visitor};

/// Pass that collapses
///(seq
///    (seq (enable A B)
///         (enable C D))
/// ..)
/// into
/// (seq (enable A B C D)
///  ..)
/// given that there are no edges between the sub-graphs induced by (enable A B) and (enable C D)
/// since in this case there is no way for these subgraphs to depend on each other
/// XXX (zhijing): I think this pass need to be changed if we add `enable` CSP style components to futil semantics
#[derive(Default)]
pub struct AutomaticPar {}

impl Visitor for AutomaticPar {
    fn name(&self) -> String {
        "automatic parallelization".to_string()
    }

    // use finish_seq so that we collapse things on the way
    // back up the tree and potentially catch more cases
    fn finish_seq(
        &mut self,
        seq: &mut ast::Seq,
        comp: &mut Component,
        _: &Context,
    ) -> VisResult {
        let mut seqs = seq.clone();
        let mut done = false;
        let mut i = 0;

        let st = &mut comp.structure;
        while !done {
            if i == &seqs.stmts.len() - 2 {
                done = true;
            }
            match (&seqs.stmts[i], &seqs.stmts[i + 1]) {
                (
                    Control::Enable { data: enables1 },
                    Control::Enable { data: enables2 },
                ) => {
                    let mut conflict = false;
                    // for every component in the first enable
                    for en_comp in &enables1.comps {
                        let idx = st.get_inst_index(en_comp)?;
                        // for every output, check if any incoming/outgoing edges
                        // contain a component in `enables2`
                        conflict |= st.graph[idx].out_ports().any(|port| {
                            let outgoing = st
                                .connected_to(idx, port.to_string())
                                .any(|(node_data, _)| {
                                    enables2
                                        .comps
                                        .contains(node_data.get_name())
                                });
                            let incoming = st
                                .connected_from(idx, port.to_string())
                                .any(|(node_data, _)| {
                                    enables2
                                        .comps
                                        .contains(node_data.get_name())
                                });
                            outgoing | incoming
                        });
                    }

                    if conflict {
                        i += 1;
                    } else {
                        let merge_enable: Vec<ast::Id> = enables1
                            .comps
                            .clone()
                            .into_iter()
                            .chain(enables2.comps.clone().into_iter())
                            .collect();
                        seqs.stmts[i] = ast::Control::enable(merge_enable);
                        seqs.stmts.remove(i + 1);
                    }
                }
                _ => continue,
            }
        }
        Ok(Action::Change(ast::Control::Seq { data: seqs }))
    }
}
