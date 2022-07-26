//! Defines common traits for methods that attempt to share components.
use crate::{analysis::GraphColoring, ir, passes::CellShare};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName,
};
use std::collections::HashMap;

///
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
        let start = std::time::Instant::now();
        self.initialize(comp, sigs);
        log::info!("{} ms", start.elapsed().as_millis());

        let cells = comp.cells.iter().filter(|c| self.cell_filter(&c.borrow()));

        // Mapping from cell names (the ir::Id's) to cell types
        let id_to_type: HashMap<ir::Id, ir::CellType> = cells
            .clone()
            .map(|cell| (cell.clone_name(), cell.borrow().prototype.clone()))
            .collect();

        // Mapping from cell type to names of all cells of that type.
        let mut cells_by_type: HashMap<ir::CellType, Vec<ir::Id>> =
            HashMap::new();
        for cell in cells {
            cells_by_type
                .entry(cell.borrow().prototype.clone())
                .or_default()
                .push(cell.clone_name())
        }

        // Maps cell type to conflict graph (will be used to perform coloring)
        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        self.set_id_to_type(id_to_type);

        log::info!("{} ms", start.elapsed().as_millis());
        self.add_conflicts(&mut graphs_by_type, &*comp.control.borrow(), false);
        log::info!("{} ms", start.elapsed().as_millis());

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
