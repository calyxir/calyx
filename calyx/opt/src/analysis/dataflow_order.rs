use super::read_write_set::ReadWriteSet;
use crate::analysis;
use calyx_ir::{self as ir};
use calyx_utils::{CalyxResult, Error};
use ir::RRC;
use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{HashMap, HashSet};

/// Mapping from the name output port to all the input ports that must be driven before it.
type WriteMap = HashMap<ir::Id, HashSet<ir::Id>>;

/// Given a set of assignment, generates an ordering that respects combinatinal
/// dataflow.
pub struct DataflowOrder {
    // Mapping from name of a primitive to its [WriteMap].
    write_map: HashMap<ir::Id, WriteMap>,
}

/// Generate a write map using a primitive definition.
fn prim_to_write_map(prim: &ir::Primitive) -> CalyxResult<WriteMap> {
    let read_together_spec = analysis::PortInterface::comb_path_spec(prim)?;
    let mut inputs = HashSet::new();
    let mut outputs: Vec<(ir::Id, bool)> = Vec::new();

    // Handle ports not mentioned in read_together specs.
    // Each remaining output ports are dependent on all remaining inputs unless it is marked as
    // @stable or is an interface port in which case it does not depend on any inputs.
    for port in &prim.signature {
        let attrs = &port.attributes;
        if attrs.get(ir::NumAttr::ReadTogether).is_some() {
            continue;
        }
        match port.direction {
            ir::Direction::Input => {
                inputs.insert(port.name());
            }
            ir::Direction::Output => outputs.push((
                port.name(),
                attrs
                    .get(ir::BoolAttr::Stable)
                    .or_else(|| attrs.get(ir::NumAttr::Done))
                    .is_some(),
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

/// Get the name of the port's cell's prototype if it is a component.
fn primitive_parent(pr: &RRC<ir::Port>) -> Option<ir::Id> {
    let port = pr.borrow();
    match &port.cell_parent().borrow().prototype {
        ir::CellType::Primitive { name, .. } => Some(*name),
        ir::CellType::Component { .. }
        | ir::CellType::ThisComponent
        | ir::CellType::Constant { .. } => None,
    }
}

impl DataflowOrder {
    pub fn new<'a>(
        primitives: impl Iterator<Item = &'a ir::Primitive>,
    ) -> CalyxResult<Self> {
        let write_map = primitives
            .map(|p| prim_to_write_map(p).map(|wm| (p.name, wm)))
            .collect::<CalyxResult<_>>()?;
        Ok(DataflowOrder { write_map })
    }

    pub fn dataflow_sort<T>(
        &self,
        assigns: Vec<ir::Assignment<T>>,
    ) -> CalyxResult<Vec<ir::Assignment<T>>>
    where
        T: ToString + Clone + Eq,
    {
        // Construct a graph where a node is an assignment and there is edge between
        // nodes if one should occur before another.
        let mut gr: DiGraph<Option<ir::Assignment<T>>, ()> = DiGraph::new();

        // Mapping from the index corresponding to an assignment to its read/write sets.
        let mut writes: HashMap<ir::Canonical, Vec<NodeIndex>> = HashMap::new();
        let mut reads: Vec<(NodeIndex, (ir::Id, ir::Canonical))> =
            Vec::with_capacity(assigns.len());

        // Assignments to the hole are not considered in the sorting.
        let mut hole_writes: Vec<ir::Assignment<T>> = Vec::new();

        // Construct the nodes that contain the assignments
        for assign in assigns {
            if assign.dst.borrow().is_hole() {
                hole_writes.push(assign)
            } else {
                let rs = ReadWriteSet::port_reads(&assign)
                    .filter_map(|p| {
                        primitive_parent(&p)
                            .map(|comp| (comp, p.borrow().canonical()))
                    })
                    .collect_vec();
                let ws = {
                    let dst = assign.dst.borrow();
                    if dst.cell_parent().borrow().is_primitive::<&str>(None) {
                        Some(dst.canonical())
                    } else {
                        None
                    }
                };
                let idx = gr.add_node(Some(assign));
                reads.extend(rs.into_iter().map(|r| (idx, r)));
                if let Some(w_can) = ws {
                    writes.entry(w_can).or_default().push(idx);
                }
            }
        }

        // Walk over the writes and add edges between all required reads
        // XXX(rachit): This probably adds a bunch of duplicate edges and in the
        // worst case makes this pass much slower than it needs to be.
        for (r_idx, (comp, canonical_port)) in reads {
            let ir::Canonical { cell: inst, port } = canonical_port;
            let dep_ports = self
                .write_map
                .get(&comp)
                .unwrap_or_else(|| {
                    panic!("Component `{}` write map is not defined", comp)
                })
                .get(&port)
                .unwrap_or_else(|| {
                    panic!(
                        "Port `{}.{}` write map is not defined",
                        comp,
                        port.clone()
                    )
                });

            dep_ports
                .iter()
                .cloned()
                .flat_map(|port| writes.get(&ir::Canonical::new(inst, port)))
                .flatten()
                .try_for_each(|w_idx| {
                    if *w_idx == r_idx {
                        Err(Error::misc(format!(
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
                .map(|idx| gr[idx].take().unwrap())
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
            Err(Error::misc(format!("Found combinational cycle:\n{}", msg)))
        }
    }
}
