use crate::analysis::{PromotionAnalysis, ReadWriteSet};
use calyx_ir as ir;
use calyx_utils::{CalyxResult, Error};
use ir::RRC;
use itertools::Itertools;
use petgraph::{
    algo,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Extract the dependency order of a list of control programs.
/// Dependencies are defined using read/write sets used in the control program.
/// The read/write sets ignore ports on constants and ThisComponent.
///
/// For example, if we have control programs C1 and C2 with read sets R1 and
/// R2 and write sets W1 and W2 respectively, we can define an order relationship:
///
/// C1 < C2 if (R1 subset of W2) and (R2 disjoint W1)
/// C1 > C2 if (R2 subset of W1) and (R1 disjoint W2)
/// C1 =!= if (R1 subset of W2) and (R2 subset of W1)
///
/// Setting `BETTER_ERR` turns on additional machinery to generate an explanation for what caused
/// the error but may require expensive computations. Turn on when cycles should be a hard error.
pub struct ControlOrder<const BETTER_ERR: bool>;

impl<const BETTER_ERR: bool> ControlOrder<BETTER_ERR> {
    fn get_cells(ports: Vec<RRC<ir::Port>>) -> impl Iterator<Item = ir::Id> {
        ports
            .into_iter()
            .filter_map(|p| {
                let cr = p.borrow().cell_parent();
                let cell = cr.borrow();
                match cell.prototype {
                    // Ignore constants and _this
                    ir::CellType::Constant { .. } => None,
                    ir::CellType::ThisComponent => None,
                    _ => Some(cell.name()),
                }
            })
            .unique()
    }

    // Filters out the constants from `cells`, while mapping the remaining `ir:Cell`s
    // to their cell name.
    fn filter_out_constants(
        cells: &[RRC<ir::Cell>],
    ) -> impl Iterator<Item = ir::Id> + '_ {
        cells
            .iter()
            .filter_map(|cell| match cell.borrow().prototype {
                ir::CellType::Constant { .. } => None,
                ir::CellType::Component { .. }
                | ir::CellType::Primitive { .. }
                | ir::CellType::ThisComponent { .. } => {
                    Some(cell.borrow().name())
                }
            })
            .unique()
    }

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
                let cells = Self::get_cells(ports);
                for cell in cells {
                    map.entry(cell).or_default().push(idx);
                }
            };

        // Compute read/write sets and add them to the maps
        for c in stmts {
            let (port_reads, port_writes) =
                ReadWriteSet::control_port_read_write_set::<true>(&c);
            let idx = gr.add_node(Some(c));
            add_cells(idx, port_reads, &mut reads);
            add_cells(idx, port_writes, &mut writes);
        }

        // Add edges between read and writes
        for (cell, r_idxs) in reads {
            if let Some(wr_idxs) = writes.get(&cell) {
                wr_idxs.iter().cartesian_product(r_idxs.iter()).for_each(
                    |(wr, r)| {
                        if wr != r {
                            gr.add_edge(*r, *wr, ());
                        }
                    },
                );
            }
        }

        if let Ok(order) = algo::toposort(&gr, None) {
            let assigns = order
                .into_iter()
                .map(|idx| gr[idx].take().unwrap())
                .collect_vec();
            Ok(assigns)
        } else {
            let mut msg = "".to_string();
            if BETTER_ERR {
                // Compute strongly connected component of the graph
                let sccs = algo::kosaraju_scc(&gr);
                let scc = sccs.iter().find(|cc| cc.len() > 1).unwrap();
                msg = scc
                    .iter()
                    .map(|idx| {
                        let con = gr[*idx].as_ref().unwrap();
                        let mut msg = ir::Printer::control_to_str(con);
                        let (port_reads, port_writes) =
                            ReadWriteSet::control_port_read_write_set::<true>(
                                con,
                            );
                        write!(
                            msg,
                            "  which reads: {}\n  and writes: {}",
                            Self::get_cells(port_reads)
                                .map(|c| c.id)
                                .join(", "),
                            Self::get_cells(port_writes)
                                .map(|c| c.id)
                                .join(", "),
                        )
                        .unwrap();
                        msg
                    })
                    .join("\n");
            }
            Err(Error::misc(format!("No possible sequential ordering. Control programs exhibit data race:\n{}", msg)))
        }
    }

    // Returns a graph of dependency for input programs.
    // IMPORTANT: we ignore assignments to done ports.
    // Input control programs are considered to have data dependency if:
    // 1. subsequent program writes to cells that previous program reads from
    // 2. subsequent program writes to cells that previous program writes to
    // 3. subsequent program reads from cells that previous program writes to
    // Furthermore, we add dependencies due to continuous assignments as well. If:
    // 4. Program writes to cell that a continuous assignment writes to or reads from.
    // 5. Program reads from a cell that a continuous assignment writes to.
    // Then the program "touches" the continuous assignments, and therefore depends
    // on all previous programs that "touched" continuous assignments as well.
    // In short, we treat continuous assignments as one big cell.
    pub fn get_dependency_graph_seq(
        stmts: impl Iterator<Item = ir::Control>,
        (cont_reads, cont_writes): (
            &Vec<ir::RRC<ir::Cell>>,
            &Vec<ir::RRC<ir::Cell>>,
        ),
        dependency: &mut HashMap<NodeIndex, Vec<NodeIndex>>,
        latency_map: &mut HashMap<NodeIndex, u64>,
    ) -> DiGraph<Option<ir::Control>, ()> {
        // The names of the cells that are read/written in continuous assignments
        let cont_read_cell_names =
            Self::filter_out_constants(cont_reads).collect_vec();
        let cont_write_cell_names =
            Self::filter_out_constants(cont_writes).collect_vec();

        // Directed graph where edges means that a control program must be run before.
        let mut gr: DiGraph<Option<ir::Control>, ()> = DiGraph::new();

        // Mapping name of cell to all the indices that read or write to it.
        let mut reads: HashMap<ir::Id, Vec<NodeIndex>> = HashMap::default();
        let mut writes: HashMap<ir::Id, Vec<NodeIndex>> = HashMap::default();

        // Stores the nodes (i.e., control stmts) that are affected by continuous
        // assignments
        let mut continuous_idxs: HashSet<NodeIndex> = HashSet::new();

        for c in stmts {
            let (cell_reads, cell_writes) =
                ReadWriteSet::control_read_write_set::<false>(&c);
            let r_cell_names = Self::filter_out_constants(&cell_reads);
            let w_cell_names = Self::filter_out_constants(&cell_writes);
            let latency = PromotionAnalysis::get_inferred_latency(&c);
            let idx = gr.add_node(Some(c));
            dependency.insert(idx, Vec::new());
            latency_map.insert(idx, latency);

            for cell in r_cell_names {
                // Checking: 3. subsequent program reads from cells that previous program writes to
                if let Some(wr_idxs) = writes.get(&cell) {
                    for wr_idx in wr_idxs {
                        if !wr_idx.eq(&idx) {
                            gr.add_edge(*wr_idx, idx, ());
                            dependency.entry(idx).or_default().push(*wr_idx);
                        }
                    }
                }

                // Checking: 5. Program reads from a cell that a continuous
                // assignment writes to.
                if cont_write_cell_names.contains(&cell) {
                    for cur_idx in continuous_idxs.iter() {
                        if !cur_idx.eq(&idx) {
                            gr.add_edge(*cur_idx, idx, ());
                            dependency.entry(idx).or_default().push(*cur_idx);
                        }
                    }
                    continuous_idxs.insert(idx);
                }
                reads.entry(cell).or_default().push(idx);
            }

            for cell in w_cell_names {
                // Checking: 2. subsequent program writes to cells that previous program writes to
                if let Some(wr_idxs) = writes.get(&cell) {
                    for wr_idx in wr_idxs {
                        if !wr_idx.eq(&idx) {
                            gr.add_edge(*wr_idx, idx, ());
                            dependency.entry(idx).or_default().push(*wr_idx);
                        }
                    }
                }

                // Checking: 1. subsequent program writes to cells that previous program reads from
                if let Some(r_idxs) = reads.get(&cell) {
                    for r_idx in r_idxs {
                        if !r_idx.eq(&idx) {
                            gr.add_edge(*r_idx, idx, ());
                            dependency.entry(idx).or_default().push(*r_idx);
                        }
                    }
                }

                // Checking: 4. Program writes to cell that a continuous assignment
                // writes to or reads from.
                if cont_write_cell_names.contains(&cell)
                    || cont_read_cell_names.contains(&cell)
                {
                    for cur_idx in continuous_idxs.iter() {
                        if !cur_idx.eq(&idx) {
                            gr.add_edge(*cur_idx, idx, ());
                            dependency.entry(idx).or_default().push(*cur_idx);
                        }
                    }
                    continuous_idxs.insert(idx);
                }

                writes.entry(cell).or_default().push(idx);
            }
        }
        gr
    }
}
