use std::collections::HashMap;

use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};

use crate::{analysis::ReadWriteSet, ir::RRC};
use crate::{
    errors::{CalyxResult, Error},
    ir::{self, CloneName},
};

/// Extract the dependency order of a list of control programs.
/// Dependencies are defined using read/write sets used in the control program.
///
/// For example, if we have control programs C1 and C2 with read sets R1 and
/// R2 and write sets W1 and W2 respectively, we can define an order relationship:
///
/// C1 < C2 if (R1 subset of W2) and (R2 disjoint W1)
/// C1 > C2 if (R2 subset of W1) and (R1 disjoint W2)
/// C1 =!= if (R1 subset of W2) and (R2 subset of W1)
pub struct ControlOrder;

impl ControlOrder {
    /// Return a total order for the control programs.
    /// Returns an error if there is a cycle
    pub fn get_total_order(
        stmts: impl Iterator<Item = ir::Control>,
    ) -> CalyxResult<Vec<ir::Control>> {
        // Directed graph where edges means that a control program must be run before.
        let mut gr: DiGraph<Option<ir::Control>, ()> = DiGraph::new();

        // Mapping name of cell to all the indices that read or write to it.
        let mut reads: HashMap<ir::Id, Vec<NodeIndex>> = HashMap::default();
        let mut writes: HashMap<ir::Id, Vec<NodeIndex>> = HashMap::default();

        let add_cells =
            |idx: NodeIndex,
             ports: Vec<RRC<ir::Port>>,
             map: &mut HashMap<ir::Id, Vec<NodeIndex>>| {
                let cells = ports
                    .into_iter()
                    .map(|p| p.borrow().cell_parent().clone_name())
                    .unique();
                for cell in cells {
                    map.entry(cell.clone()).or_default().push(idx);
                }
            };

        // Compute read/write sets and add them to the maps
        for c in stmts {
            let (port_reads, port_writes) =
                ReadWriteSet::control_port_read_write_set(&c);
            let idx = gr.add_node(Some(c));
            add_cells(idx, port_reads, &mut reads);
            add_cells(idx, port_writes, &mut writes);
        }

        // Add edges between read and writes
        for (cell, r_idxs) in reads {
            if let Some(wr_idxs) = writes.get(&cell) {
                wr_idxs.iter().cartesian_product(r_idxs.iter()).for_each(
                    |(wr, r)| {
                        gr.add_edge(*r, *wr, ());
                    },
                );
            }
        }

        if let Ok(order) = algo::toposort(&gr, None) {
            let assigns = order
                .into_iter()
                .map(|idx| std::mem::replace(&mut gr[idx], None).unwrap())
                .collect_vec();
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
                    ir::Printer::control_to_str(gr[*idx].as_ref().unwrap())
                })
                .join("\n");
            Err(Error::misc(format!("No possible sequential ordering. Control programs exhibit data race:\n{}", msg)))
        }
    }
}
