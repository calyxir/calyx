//! Defines common traits for methods that attempt to share components.
use crate::{analysis::GraphColoring, ir};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName, RRC,
};
use std::collections::{HashMap, HashSet};

/// A trait for implementing passes that want to share components
/// by building a conflict graph and performing graph coloring
/// to minimize the number of used components.
///
/// You must implement the functions:
///  - [ShareComponents::lookup_node_conflicts]
///  - [ShareComponents::cell_filter]
///  - [ShareComponents::set_rewrites]
///  - [ShareComponents::get_rewrites]
///
/// Given these functions, the trait [Visitor] will automatically be
/// implemented for your struct.
///
/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use [ScheduleConflicts] to find groups/invokes that run in parallel with each other
///  - for each group/invoke, `G` that runs in parallel with another group/invoke `H`, add edges between each
///  cell in the sets `lookup_node_conflicts(G)` and `lookup_node_conflicts(H)`.
///  - add conflicts between cells where for `c0 != c1`
///  - call `custom_conflicts` to insert pass specific conflict edges
///  - perform graph coloring using `self.ordering` to define the order of the greedy coloring
///  - use coloring to rewrite group assignments, continuous assignments, and conditional ports.
pub trait ShareComponents {
    /// Initialize the structure. This function is called at the very beginning of the traversal
    /// before anything else.
    fn initialize(
        &mut self,
        _component: &ir::Component,
        _library_signatures: &ir::LibrarySignatures,
    ) {
        // nothing
    }

    /// Return a vector of conflicting cell names for a the group `group_name`.
    /// These are the names of the cells that conflict if their groups are
    /// run in parallel.
    fn lookup_node_conflicts(
        &self,
        node_name: &ir::Id,
    ) -> HashMap<&ir::CellType, HashSet<&ir::Id>>;

    /// Given a cell and the library signatures, this function decides if
    /// this cell is relevant to the current sharing pass or not. This
    /// is used to filter out irrelevant cells.
    fn cell_filter(&self, cell: &ir::Cell) -> bool;

    /// The definition of cell equality. Cells will only be replaced with
    /// a cell that is equal to it according to this function. The default
    /// implementation is to compare the prototypes of the cell.
    fn cell_equality(&self, cell0: &ir::Cell, cell1: &ir::Cell) -> bool {
        cell0.prototype == cell1.prototype
    }

    /// Set the list of rewrites.
    fn set_rewrites(&mut self, rewrites: HashMap<ir::Id, RRC<ir::Cell>>);

    /// Get the list of rewrites.
    fn get_rewrites(&self) -> &HashMap<ir::Id, RRC<ir::Cell>>;

    fn set_id_to_type(&mut self, id_to_type: HashMap<ir::Id, ir::CellType>);

    ///
    fn build_conflict_graph(
        &self,
        graphs_by_type: &mut HashMap<ir::CellType, GraphColoring<ir::Id>>,
        c: &ir::Control,
        is_in_par: bool,
    ) -> HashMap<&ir::CellType, HashSet<&ir::Id>>;
}

impl<T: ShareComponents> Visitor for T {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        self.initialize(comp, sigs);

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

        self.build_conflict_graph(
            &mut graphs_by_type,
            &*comp.control.borrow(),
            false,
        );

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
