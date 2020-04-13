use crate::errors;
use crate::lang::component::Component;
use crate::lang::{
    ast, ast::Control, context::Context, structure::StructureGraph,
};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};
use std::collections::HashMap;
/// Pass that collapses
/// ```
/// (par (enable A B)
///      (enable C D))
///      ..)
/// ```
/// into
/// ```
/// (par (enable A B C D)
///      ..)
/// ```
///
/// when the sub-graphs induced by (enable A B) and (enable C D) have no common
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
/// (par (enable A B) (enable C D))
/// ```
/// is equivalent to
/// ```
/// (par (enable A B C D))
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
/// then `par` should be collapsed to:
/// ```
/// ╭─╮   ╭──╮   ╭─╮
/// │A│──>│id│──>│C│
/// ╰┬╯   ╰──╯   ╰┬╯
///  │            │
///  │            │
///  v            v
/// ╭─╮          ╭─╮
/// │B│          │D│
/// ╰─╯          ╰─╯
/// ```
/// which we can represented as
/// ```
/// (par (enable A B C D))
/// ```
/// and replace the control of `enable A C` with
/// ```
/// (enable A id C)

#[derive(Default)]
pub struct RemovePar {
    pub edge_clear: HashMap<ast::Id, Vec<ast::Id>>,
}

impl Named for RemovePar {
    fn name() -> &'static str {
        "remove-par"
    }

    fn description() -> &'static str {
        "automatically merges some parallel enables"
    }
}

// Check if two given components have an edge between them.
fn resolve_conflicts(
    edge_clear: &mut HashMap<ast::Id, Vec<ast::Id>>,
    comps1: &[ast::Id],
    comps2: &[ast::Id],
    structure: &mut StructureGraph,
    ctx: &Context,
) -> Result<(), errors::Error> {
    let mut add_structure = |st: &mut StructureGraph,
                             comp1: &ast::Id,
                             port1: &ast::Id,
                             comp2: &ast::Id,
                             port2: &ast::Id,
                             width: u64| {
        let name = format!("{}_{}_id", comp1.to_string(), comp2.to_string());
        //TODO: check if id already exists
        let id_comp =
            ctx.instantiate_primitive(&name, &"std_id".into(), &[width])?;
        let id = st.add_primitive(&name.into(), "std_id", &id_comp, &[width]);

        let idx1 = st.get_inst_index(comp1)?;
        let idx2 = st.get_inst_index(comp2)?;

        st.insert_edge(idx1, port1, id, "in")?;
        st.insert_edge(id, "out", idx2, port2)?;

        if edge_clear.contains_key(&comp1) {
            edge_clear.get_mut(&comp1).unwrap().push(comp2.clone());
        } else {
            edge_clear.insert(comp1.clone(), vec![comp2.clone()]);
        }
        st.remove_edge(idx1, port1, idx2, port2)
    };
    let st_origin = structure.clone();
    for idx2 in comps2
        .iter()
        .map(|en_comp| st_origin.get_inst_index(en_comp))
        // If any of the get_inst_index failed, return the error.
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
    {
        for port2 in st_origin.graph[idx2].out_ports() {
            for (node_data, edge_data) in
                st_origin.connected_to(idx2, port2.to_string())
            {
                let comp_src = st_origin.graph[idx2].get_name();
                let comp_dest = node_data.get_name();
                if comps1.contains(comp_dest) {
                    let port_src: ast::Id = edge_data.src.clone().into();
                    let port_dest: ast::Id = edge_data.dest.clone().into();
                    add_structure(
                        structure,
                        comp_src,
                        &port_src,
                        comp_dest,
                        &port_dest,
                        edge_data.width,
                    )?;
                }
            }
        }
        for port2 in st_origin.graph[idx2].in_ports() {
            for (node_data, edge_data) in
                st_origin.connected_from(idx2, port2.to_string())
            {
                let comp_dest = st_origin.graph[idx2].get_name();
                let comp_src = node_data.get_name();
                if comps1.contains(comp_src) {
                    let port_src: ast::Id = edge_data.src.clone().into();
                    let port_dest: ast::Id = edge_data.dest.clone().into();
                    add_structure(
                        structure,
                        comp_src,
                        &port_src,
                        comp_dest,
                        &port_dest,
                        edge_data.width,
                    )?;
                }
            }
        }
    }

    Ok(())
}

impl Visitor for RemovePar {
    fn finish_par(
        &mut self,
        par: &ast::Par,
        _comp: &mut Component,
        _ctx: &Context,
    ) -> VisResult {
        // get first enable statement and its index. this will act
        // as the accumulator for the statement that we are collapsing
        // things into
        let (start, cmp_acc) = match par.stmts.iter().enumerate().find_map(
            |(i, stmt)| match stmt {
                Control::Enable { data } => Some((i, data.clone())),
                _ => None,
            },
        ) {
            Some((s, c)) => (s, c),
            None => return Ok(Action::Continue),
        };

        // start interation from the start+1 because the start is in `cmp_stmt`.
        // This loop will keep trying to merge adjacent parallels as long as
        // there are no conflicts. When it detects a conflict, it attempts
        // to
        for stmt in par.stmts[start + 1..].iter() {
            if let Control::Enable { data: enables2 } = stmt {
                // for every component in the `enables2` we check if any
                // incoming/outgoing edge has an endpoint in `cmp_acc`
                resolve_conflicts(
                    &mut self.edge_clear,
                    &cmp_acc.comps,
                    &enables2.comps,
                    &mut _comp.structure,
                    &_ctx,
                )?;
            }
        }

        Ok(Action::Continue)
    }
}
