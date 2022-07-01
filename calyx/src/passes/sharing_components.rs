//! Defines common traits for methods that attempt to share components.
use crate::{
    analysis::{GraphColoring, ScheduleConflicts},
    ir,
};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName, RRC,
};
use itertools::Itertools;
use std::collections::HashMap;

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
    fn lookup_node_conflicts(&self, node_name: &ir::Id) -> Vec<&ir::Id>;

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

    /// Called after the initial conflict graph is constructed.
    /// This function lets you add custom conflicts to the graph
    /// before graph coloring is performed.
    fn custom_conflicts<F>(&self, _comp: &ir::Component, _add_conflicts: F)
    where
        F: FnMut(Vec<ir::Id>),
    {
    }

    /// Set the list of rewrites.
    fn set_rewrites(&mut self, rewrites: HashMap<ir::Id, RRC<ir::Cell>>);

    /// Get the list of rewrites.
    fn get_rewrites(&self) -> &HashMap<ir::Id, RRC<ir::Cell>>;
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

        let id_to_type: HashMap<ir::Id, ir::CellType> = cells
            .clone()
            .map(|cell| (cell.clone_name(), cell.borrow().prototype.clone()))
            .collect();

        let mut cells_by_type: HashMap<ir::CellType, Vec<ir::Id>> =
            HashMap::new();
        for cell in cells {
            cells_by_type
                .entry(cell.borrow().prototype.clone())
                .or_default()
                .push(cell.clone_name())
        }

        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());
        let node_conflicts = par_conflicts
            .all_conflicts()
            .into_grouping_map_by(|(g1, _)| g1.clone())
            .fold(
                HashMap::new(),
                |mut acc: HashMap<ir::CellType, Vec<ir::Id>>,
                 _,
                 (_, conflicted_node)| {
                    for conflict in self.lookup_node_conflicts(&conflicted_node)
                    {
                        acc.entry(id_to_type[conflict].clone())
                            .or_default()
                            .push(conflict.clone())
                    }
                    acc
                },
            );

        node_conflicts
            .into_iter()
            .for_each(|(node, conflict_node_b)| {
                for a in self.lookup_node_conflicts(&node) {
                    let g = graphs_by_type.get_mut(&id_to_type[a]).unwrap();
                    if let Some(confs) = conflict_node_b.get(&id_to_type[a]) {
                        for b in confs {
                            if a != b {
                                g.insert_conflict(a, b);
                            }
                        }
                    }
                }
            });

        // add custom conflicts
        self.custom_conflicts(comp, |confs: Vec<ir::Id>| {
            for (a, b) in confs.iter().tuple_combinations() {
                if id_to_type[a] == id_to_type[b] {
                    if let Some(g) = graphs_by_type.get_mut(&id_to_type[a]) {
                        g.insert_conflict(a, b)
                    }
                }
            }
        });

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
