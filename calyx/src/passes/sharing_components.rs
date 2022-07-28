//! Defines common traits for methods that attempt to share components.
use crate::{
    analysis::{GraphColoring, LiveRangeAnalysis},
    ir,
    passes::CellShare,
};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName,
};
use itertools::Itertools;

use std::collections::{HashMap, HashSet};

const NODE_ID: &str = "NODE_ID";

/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use [ScheduleConflicts] to find groups/invokes that run in parallel with each other
///  - for each tuple combination of cells, c1 and c2
///  - first determine if their live ranges overlap. If so, then insert a conflict between
///  c1 and c2
///  - if c1 and c2 don't have overlapping live ranges, check if c1's live
///  range overlaps with any nodes (groups/invokes) running in parallale to
///  c2, or vice versa.
///  - perform graph coloring using `self.ordering` to define the order of the greedy coloring
///  - use coloring to rewrite group assignments, continuous assignments, and conditional ports.
impl Visitor for CellShare {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.initialize(comp, sigs);

        let cells = comp.cells.iter().filter(|c| self.cell_filter(&c.borrow()));

        // Mapping from cell type to names of all cells of that type.
        let mut cells_by_type: HashMap<ir::CellType, Vec<ir::Id>> =
            HashMap::new();
        for cell in cells {
            cells_by_type
                .entry(cell.borrow().prototype.clone())
                .or_default()
                .push(cell.clone_name());
        }

        // Maps cell type to conflict graph (will be used to perform coloring)
        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .clone()
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        // Give each control statement a unique "NODE_ID" attribute.
        compute_unique_ids(&mut comp.control.borrow_mut(), 0);

        // live_once_map maps control statement ids to all cells that were
        // live at least once within the control statement. The cells are organized
        // by cell type. Furthermore, only control statements that are direct
        // children of par blocks are included in this map.
        let mut live_once_map = HashMap::new();
        // Maps every control statement that is a direct child of a par block to
        // its parent par block. (maps id number to id number)
        let mut par_thread_map = HashMap::new();
        // build live_once_map and par_thread_map
        self.live.get_live_once_data(
            &mut live_once_map,
            &mut par_thread_map,
            &*comp.control.borrow(),
            false,
            false,
        );

        // maps celltype to a map. The map contains all cells of type celltype
        // that appear in live_once_map, mapped to all of the direct children
        // of pars in which the cell is live. In other words, it restructures the
        // data in live_once_map.
        let mut live_once_cellmap =
            LiveRangeAnalysis::get_cell_to_control(live_once_map);

        // Essentially makes sure that all cells in the component are represented
        // in live_once_cellmap. Certain cells may not be located within a par block
        // and therefore not represented live_once_map. In this case, we just map
        // this cell to an empty HashSet.
        /*for (cell_type, cell_names) in cells_by_type {
            let name_to_live = live_once_cellmap.entry(cell_type).or_default();
            for name in cell_names {
                if matches!(name_to_live.get(&name), None) {
                    name_to_live.insert(name, HashSet::new());
                }
            }
        }*/

        // Maps celltype to map, which maps cells to groups/invokes in which the cell is live.
        let live_cell_map: HashMap<
            ir::CellType,
            HashMap<ir::Id, HashSet<&ir::Id>>,
        > = self.live.get_reverse();

        for (cell_type, cells) in &cells_by_type {
            // Run remove_dead_cells before this cell-share pass.
            let g = graphs_by_type.get_mut(&cell_type).unwrap();
            let cell_to_nodes = live_cell_map.get(&cell_type).unwrap();
            let live_once_map =
                live_once_cellmap.entry(cell_type.clone()).or_default();
            for (a, b) in cells.iter().tuple_combinations() {
                // nodes (groups/invokes) in which a is live
                let a_live: &HashSet<&ir::Id> = cell_to_nodes.get(a).unwrap();
                // nodes (groups/invokes) in which b is live
                let b_live: &HashSet<&ir::Id> = cell_to_nodes.get(b).unwrap();
                if !a_live.is_disjoint(b_live) {
                    g.insert_conflict(a, b);
                    continue;
                }
                if let Some(live_once_a) = live_once_map.get(a) {
                    if let Some(live_once_b) = live_once_map.get(b) {
                        'outer: for live_a in live_once_a {
                            for live_b in live_once_b {
                                // a and b were live within the same par block but not within
                                // the same child, then insert conflict.
                                if live_a != live_b
                                    && par_thread_map.get(live_a).unwrap()
                                        == par_thread_map.get(live_b).unwrap()
                                {
                                    g.insert_conflict(a, b);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        /*for (cell_type, cell_map) in &live_once_cellmap {
            // Run remove_dead_cells before this cell-share pass.
            let g = graphs_by_type.get_mut(&cell_type).unwrap();
            let cell_to_nodes = live_cell_map.get(&cell_type).unwrap();
            for ((a, live_once_a), (b, live_once_b)) in
                cell_map.iter().tuple_combinations()
            {
                // nodes (groups/invokes) in which a is live
                let a_live: &HashSet<&ir::Id> = cell_to_nodes.get(a).unwrap();
                // nodes (groups/invokes) in which b is live
                let b_live: &HashSet<&ir::Id> = cell_to_nodes.get(b).unwrap();
                if !a_live.is_disjoint(b_live) {
                    g.insert_conflict(a, b);
                    continue;
                }
                // live_once_a contains all the childrens of par control statements
                // in which  a was live at one point.
                'outer: for live_a in live_once_a {
                    for live_b in live_once_b {
                        // a and b were live within the same par block but not within
                        // the same child, then insert conflict.
                        if live_a != live_b
                            && par_thread_map.get(live_a).unwrap()
                                == par_thread_map.get(live_b).unwrap()
                        {
                            g.insert_conflict(a, b);
                            break 'outer;
                        }
                    }
                }
            }
        }*/

        // perform graph coloring to rename the cells
        let mut coloring: ir::rewriter::CellRewriteMap = HashMap::new();
        for graph in graphs_by_type.values() {
            if graph.has_nodes() {
                coloring.extend(
                    graph
                        .color_greedy()
                        .iter()
                        .map(|(a, b)| (a.clone(), comp.find_cell(&b).unwrap())),
                );
            }
        }

        // Rewrite assignments using the coloring generated.
        let empty_map: ir::rewriter::PortRewriteMap = HashMap::new();
        let rewriter = ir::Rewriter::new(&coloring, &empty_map);
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| rewriter.get(port));
        });

        // Rewrite control uses of ports
        rewriter.rewrite_control(
            &mut *comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        Ok(Action::Stop)
    }
}

// Very similar to the domination map one-- should reuse code, instead of copy+paste
fn compute_unique_ids(con: &mut ir::Control, mut cur_state: u64) -> u64 {
    match con {
        ir::Control::Enable(ir::Enable { attributes, .. })
        | ir::Control::Invoke(ir::Invoke { attributes, .. }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state + 1
        }
        ir::Control::Par(ir::Par {
            stmts, attributes, ..
        })
        | ir::Control::Seq(ir::Seq {
            stmts, attributes, ..
        }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state += 1;
            stmts.iter_mut().for_each(|stmt| {
                let new_state = compute_unique_ids(stmt, cur_state);
                cur_state = new_state;
            });
            cur_state
        }
        ir::Control::If(ir::If {
            tbranch,
            fbranch,
            attributes,
            ..
        }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state += 1;
            cur_state = compute_unique_ids(tbranch, cur_state);
            cur_state = compute_unique_ids(fbranch, cur_state);
            cur_state + 1
        }
        ir::Control::While(ir::While {
            body, attributes, ..
        }) => {
            attributes.insert(NODE_ID, cur_state);
            cur_state += 1;
            compute_unique_ids(body, cur_state)
        }
        ir::Control::Empty(_) => cur_state,
    }
}
