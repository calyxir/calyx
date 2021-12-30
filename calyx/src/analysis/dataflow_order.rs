use std::collections::HashMap;

use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};

use super::read_write_set::ReadWriteSet;
use crate::{
    errors::{CalyxResult, Error},
    ir::{self, CloneName},
};

/// Given a set of assignment, generates an ordering that respects combinatinal
/// dataflow.
pub struct DataflowOrder;

/// Returns true iff the given port is a sequential output and therefore should
/// not be used in the ordering decision.
fn is_seq_port(port: &ir::Port) -> bool {
    port.direction == ir::Direction::Output
        && port.attributes.get("stable").is_some()
}

impl DataflowOrder {
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

    /// Return a sorted vector of assignments in dataflow order.
    pub fn dataflow_sort(
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
