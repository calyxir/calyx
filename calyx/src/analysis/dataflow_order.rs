use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};

use super::read_write_set::ReadWriteSet;
use crate::errors::{CalyxResult, Error};
use crate::ir;
use crate::{analysis, ir::RRC};

/// Mapping from the name output port to all the input ports that must be driven before it.
type WriteMap = HashMap<ir::Id, HashSet<ir::Id>>;

/// Given a set of assignment, generates an ordering that respects combinatinal
/// dataflow.
pub struct DataflowOrder {
    // Mapping from name of a primitive to its [WriteMap].
    write_map: HashMap<ir::Id, WriteMap>,
}

fn to_write_map(prim: &ir::Primitive) -> CalyxResult<WriteMap> {
    let read_together_spec = analysis::ReadWriteSpec::read_together_spec(prim)?;
    let mut inputs = HashSet::new();
    let mut outputs: Vec<(ir::Id, bool)> = Vec::new();

    // Handle ports not mentioned in read_together specs.
    // Each remaining output ports are dependent on all remaining inputs unless it is marked as
    // @stable in which case it does not depend on any inputs.
    for port in &prim.signature {
        if port.attributes.get("read_together").is_some() {
            continue;
        }
        match port.direction {
            ir::Direction::Input => {
                inputs.insert(port.name.clone());
            }
            ir::Direction::Output => outputs.push((
                port.name.clone(),
                port.attributes.get("stable").is_some(),
            )),
            ir::Direction::Inout => {
                unreachable!("Primitive ports should not be inout")
            }
        }
    }
    let all_ports: WriteMap = outputs
        .into_iter()
        .map(|(out, stable)| {
            // Stable ports don't depend on anything
            if stable {
                (out, HashSet::new())
            } else {
                (out, inputs.clone())
            }
        })
        .chain(read_together_spec)
        .collect();
    Ok(all_ports)
}

fn parent_port(pr: &RRC<ir::Port>) -> Option<(ir::Id, (ir::Id, ir::Id))> {
    let port = pr.borrow();
    port.cell_parent()
        .borrow()
        .type_name()
        .cloned()
        .map(|pr| (pr, port.canonical()))
}

impl DataflowOrder {
    pub fn new<'a>(
        primitives: impl Iterator<Item = &'a ir::Primitive>,
    ) -> CalyxResult<Self> {
        let write_map = primitives
            .map(|p| to_write_map(p).map(|wm| (p.name.clone(), wm)))
            .collect::<CalyxResult<_>>()?;
        Ok(DataflowOrder { write_map })
    }

    pub fn dataflow_sort(
        &self,
        assigns: Vec<ir::Assignment>,
    ) -> CalyxResult<Vec<ir::Assignment>> {
        // Construct a graph where a node is an assignment and there is edge between
        // nodes if one should occur before another.
        let mut gr: DiGraph<Option<ir::Assignment>, ()> = DiGraph::new();

        // Mapping from the index corresponding to an assignment to its read/write sets.
        let mut writes: HashMap<(ir::Id, ir::Id), Vec<NodeIndex>> =
            HashMap::new();
        let mut reads: Vec<(NodeIndex, (ir::Id, (ir::Id, ir::Id)))> =
            Vec::with_capacity(assigns.len());

        // Assignments to the hole are not considered in the sorting.
        let mut hole_writes: Vec<ir::Assignment> = Vec::new();

        // Construct the nodes that contain the assignments
        for assign in assigns {
            if assign.dst.borrow().is_hole() {
                hole_writes.push(assign)
            } else {
                let rs = ReadWriteSet::port_reads(&assign)
                    .filter_map(|p| parent_port(&p))
                    .collect_vec();
                let ws = assign.dst.borrow().canonical();
                let idx = gr.add_node(Some(assign));
                reads.extend(rs.into_iter().map(|r| (idx, r)));
                writes.entry(ws).or_default().push(idx);
            }
        }

        // Walk over the writes and add edges between all required reads
        for (r_idx, (prim, (inst, port))) in reads {
            let dep_ports = self
                .write_map
                .get(&prim)
                .unwrap_or_else(|| {
                    panic!("Component `{}` write map is not defined", prim)
                })
                .get(&port)
                .unwrap_or_else(|| {
                    panic!("Port `{}.{}` write map is not defined", prim, port)
                });

            dep_ports
                .iter()
                .cloned()
                .flat_map(|port| writes.get(&(inst.clone(), port)))
                .flatten()
                .try_for_each(|w_idx| {
                    if *w_idx == r_idx {
                        Err(Error::Misc(format!(
                            "Assignment depends on itself: {}",
                            ir::Printer::assignment_to_str(
                                gr[*w_idx].as_ref().unwrap()
                            )
                        )))
                    } else {
                        gr.add_edge(*w_idx, r_idx, ());
                        Ok(())
                    }
                })?;
        }

        // Generate a topological ordering
        if let Ok(order) = algo::toposort(&gr, None) {
            let mut assigns = order
                .into_iter()
                .map(|idx| std::mem::replace(&mut gr[idx], None).unwrap())
                .collect_vec();
            assigns.append(&mut hole_writes);
            Ok(assigns)
        } else {
            // Compute strongly connected component of the graph
            let sccs = algo::kosaraju_scc(&gr);
            let scc = sccs
                .iter()
                .find(|cc| cc.len() > 1)
                .expect("All combinational cycles are self loops");
            let msg = scc
                .iter()
                .map(|idx| {
                    ir::Printer::assignment_to_str(gr[*idx].as_ref().unwrap())
                })
                .join("\n");
            Err(Error::Misc(format!("Found combinational cycle:\n{}", msg)))
        }
    }
}
