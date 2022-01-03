use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};

use super::read_write_set::ReadWriteSet;
use crate::analysis;
use crate::errors::{CalyxResult, Error};
use crate::ir::{self, CloneName};

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
                unreachable!("Primitive ports should not be in-out")
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

/// Returns true iff the given port is a sequential output and therefore should
/// not be used in the ordering decision.
fn is_seq_port(port: &ir::Port) -> bool {
    port.direction == ir::Direction::Output
        && port.attributes.get("stable").is_some()
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

    /// Get the [NodeIndex] associated with the provided `name`. If `name` has
    /// not been added to the graph yet, add it and return the index.
    fn get_index(
        name: ir::Id,
        rev_map: &mut HashMap<ir::Id, NodeIndex>,
        gr: &mut DiGraph<ir::Id, ()>,
    ) -> NodeIndex {
        if let Some(idx) = rev_map.get(&name) {
            *idx
        } else {
            // Add cell to the graph
            let idx = gr.add_node(name.clone());
            rev_map.insert(name, idx);
            idx
        }
    }

    pub fn sort(&self, assigns: Vec<ir::Assignment>) -> Vec<ir::Assignment> {
        // Construct a graph where a node is an assignment and there is edge between
        // nodes if one should occur before another.
        let mut gr: DiGraph<ir::Assignment, ()> = DiGraph::new();

        // Mapping from the index corresponding to an assignment to its read/write sets.
        let mut reads: HashMap<(ir::Id, ir::Id), NodeIndex> = HashMap::new();
        let mut writes: HashMap<NodeIndex, (ir::Id, ir::Id)> = HashMap::new();

        // Assignments to the hole are not considered in the sorting.
        let mut hole_writes: Vec<ir::Assignment> = Vec::new();

        // Construct the nodes that contain the assignments
        for assign in assigns {
            if assign.dst.borrow().is_hole() {
                hole_writes.push(assign)
            } else {
                let rs = ReadWriteSet::port_reads(&assign)
                    .map(|p| p.borrow().canonical())
                    .collect_vec();
                let ws = assign.dst.borrow().canonical();
                let idx = gr.add_node(assign);
                reads.extend(rs.into_iter().map(|r| (r, idx)).into_iter());
                writes.insert(idx, ws);
            }
        }

        // Walk over the writes and add edges between all required reads
        for (w_idx, (prim, port)) in writes {
            let dep_ports = self
                .write_map
                .get(&prim)
                .expect(&format!("Primitive {} write map is not defined", prim))
                .get(&port)
                .expect(&format!(
                    "Port {}.{} write map is not defined",
                    prim, port
                ));

            for port in dep_ports.into_iter().cloned() {
                if let Some(r_idx) = reads.get(&(prim.clone(), port)) {
                    gr.add_edge(*r_idx, w_idx, ());
                }
            }
        }

        // Generate a topological ordering
        if let Ok(order) = algo::toposort(&gr, None) {
            let mut assigns = Vec::new();
            for idx in order {
                assigns.push(gr.remove_node(idx).unwrap())
            }
            assigns
        } else {
            todo!()
            /* let msg = assign_map
                .values()
                .flatten()
                .map(ir::Printer::assignment_to_str)
                .join("\n");
            Err(Error::Misc(format!("Found combinational cycle:\n{}", msg))) */
        }
    }

    /// Return a sorted vector of assignments in dataflow order.
    pub fn dataflow_sort(
        &self,
        assigns: Vec<ir::Assignment>,
    ) -> CalyxResult<Vec<ir::Assignment>> {
        let mut gr: DiGraph<ir::Id, ()> = DiGraph::new();
        let mut rev_map: HashMap<ir::Id, NodeIndex> = HashMap::new();
        // Map from name of cell to the assignments that write to its ports.
        let mut assign_map: HashMap<NodeIndex, Vec<ir::Assignment>> =
            HashMap::new();
        // Dummy node for writes to inputs of sequential cells.
        let dummy = ir::Id::from("@dummy");
        rev_map.insert(dummy.clone(), gr.add_node(ir::Id::from("@dummy")));
        assign_map.entry(rev_map[&dummy]).or_default();
        // Assignments to the hole
        let mut hole_writes: Vec<ir::Assignment> = Vec::new();
        for assign in assigns {
            // Hole are always placed at the end
            if assign.dst.borrow().is_hole() {
                hole_writes.push(assign);
                continue;
            }

            // Get the node index for the cell being written to.
            let wr_index = if is_seq_port(&assign.dst.borrow()) {
                rev_map[&dummy]
            } else {
                Self::get_index(
                    assign.dst.borrow().cell_parent().clone_name(),
                    &mut rev_map,
                    &mut gr,
                )
            };

            // Add edges between write cell and read cells
            ReadWriteSet::port_reads(&assign)
                .filter(|pr| !is_seq_port(&pr.borrow()))
                .map(|pr| pr.borrow().cell_parent().clone_name())
                .unique()
                .for_each(|c| {
                    let read_idx = Self::get_index(c, &mut rev_map, &mut gr);
                    // Self loops are not allowed.
                    if read_idx != wr_index {
                        gr.add_edge(read_idx, wr_index, ());
                    }
                });

            // Add the write to the assignment map
            assign_map.entry(wr_index).or_default().push(assign);
        }

        // Perform Topological sort and return assignments in the order of cells
        if let Ok(ordering) = algo::toposort(&gr, None) {
            Ok(ordering
                .into_iter()
                .flat_map(|idx| assign_map.remove(&idx))
                .flatten()
                .chain(hole_writes.into_iter())
                .collect())
        } else {
            let msg = assign_map
                .values()
                .flatten()
                .map(ir::Printer::assignment_to_str)
                .join("\n");
            Err(Error::Misc(format!("Found combinational cycle:\n{}", msg)))
        }
    }
}
