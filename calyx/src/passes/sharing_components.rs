//! Defines common traits for methods that attempt to share components.
use crate::{
    analysis::{GraphColoring, ScheduleConflicts},
    ir,
    passes::CellShare,
};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName,
};
use itertools::Itertools;

use std::collections::{HashMap, HashSet};

/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use [ScheduleConflicts] to find groups/invokes that run in parallel with each other
///  - for each group/invoke, `G` that runs in parallel with another group/invoke `H`, add edges between each
///  cell in the sets `lookup_node_conflicts(G)` and `lookup_node_conflicts(H)`.
///  - for each grou/invoke `G`, it adds edges between the cells in `lookup_node_conflicts(G)`.
///  - add conflicts between cells where for `c0 != c1`
///  - call `custom_conflicts` to insert pass specific conflict edges
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

        // get all of the invokes and enables.
        let mut invokes_enables = HashSet::new();
        get_invokes_enables(&comp.control.borrow(), &mut invokes_enables);

        // conflict (a,b) is in par_conflicts if a and b run in parallel w/ each other
        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        // building map to get par_conflicts
        // maps nodes (enables/invokes) to the set of nodes that run in parallel
        // with it
        let mut par_conflicts_map: HashMap<ir::Id, HashSet<&ir::Id>> =
            HashMap::new();
        for node in invokes_enables {
            par_conflicts_map
                .insert(node.clone(), par_conflicts.conflicts_with(&node));
        }

        // Map from cell type to map, which itself maps cell names to the nodes (groups
        // invokes) which are live at the given cell.
        let live_cell_map: HashMap<
            ir::CellType,
            HashMap<ir::Id, HashSet<&ir::Id>>,
        > = self.live.get_reverse();

        for (cell_type, cells) in cells_by_type {
            let g = graphs_by_type.get_mut(&cell_type).unwrap();
            // This assumes that there are no dead cells. If dead cells exist,
            // this unwrap() could return an error
            // cell_to_nodes is a map from cells of type cell_type to the nodes (groups/invokes)
            // in which each cell is live
            let cell_to_nodes = live_cell_map.get(&cell_type).unwrap();
            // Going through all possible cell conflicts
            for (a, b) in cells.iter().tuple_combinations() {
                // First check if a and b are ever alive at the same node (i.e. group or invoke)
                let a_live: &HashSet<&ir::Id> = cell_to_nodes.get(a).unwrap();
                let b_live: &HashSet<&ir::Id> = cell_to_nodes.get(b).unwrap();
                if !a_live.is_disjoint(b_live) {
                    g.insert_conflict(a, b);
                    continue;
                }
                // Check if b is alive at any nodes (groups/invokes)
                // running in parallel to the nodes in which a is live, or vice versa.
                // The if statement is there for efficiency.
                if a_live.len() <= b_live.len() {
                    for a_group in a_live {
                        let par_confs = par_conflicts_map.get(a_group).unwrap();
                        if !par_confs.is_disjoint(b_live) {
                            g.insert_conflict(a, b);
                            continue;
                        }
                    }
                } else {
                    for b_group in b_live {
                        let par_confs = par_conflicts_map.get(b_group).unwrap();
                        if !par_confs.is_disjoint(a_live) {
                            g.insert_conflict(a, b);
                            continue;
                        }
                    }
                }
            }
        }

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

//Gets the names of all the cells invoked (using an invoke control statement)
//in control c, and adds them to hs.
fn get_invokes_enables(c: &ir::Control, hs: &mut HashSet<ir::Id>) {
    match c {
        ir::Control::Empty(_) => (),
        ir::Control::Enable(ir::Enable { group, .. }) => {
            hs.insert(group.borrow().name().clone());
        }
        ir::Control::Invoke(ir::Invoke { comp, .. }) => {
            hs.insert(comp.borrow().name().clone());
        }
        ir::Control::Par(ir::Par { stmts, .. })
        | ir::Control::Seq(ir::Seq { stmts, .. }) => {
            for stmt in stmts {
                get_invokes_enables(stmt, hs);
            }
        }
        ir::Control::If(ir::If {
            tbranch, fbranch, ..
        }) => {
            get_invokes_enables(tbranch, hs);
            get_invokes_enables(fbranch, hs);
        }
        ir::Control::While(ir::While { body, .. }) => {
            get_invokes_enables(body, hs);
        }
    }
}
