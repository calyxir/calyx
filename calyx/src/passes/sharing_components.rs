use crate::{
    analysis::{GraphColoring, ScheduleConflicts},
    ir,
};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName, RRC,
};
use itertools::Itertools;
use std::{collections::HashMap, rc::Rc};

/// A trait for implementing passes that want to share components
/// by building a conflict graph and performing graph coloring
/// to minimize the number of used components.
///
/// You must implement the functions:
///  - `lookup_group_conflicts`
///  - `cell_filter`
///  - `set_rewrites`
///  - `get_rewrites`
///
/// Given these functions, the trait `Visitor` will automatically be
/// implemented for your struct.
///
/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use `ScheduleConflicts` to find groups that run in parallel with each other
///  - for each group, `G` that runs in parallel with another group `H`, add edges between
///  each cell in the sets `lookup_group_conflicts(G)` and `lookup_group_conflicts(H)`.
///  - add conflicts between cells where for `c0 != c1`
///  - call `custom_conflicts` to insert pass specific conflict edges
///  - perform graph coloring using `self.ordering` to define the order of the greedy coloring
///  - use coloring to rewrite group assignments, continuous assignments, and conditional ports.
pub trait ShareComponents {
    /// Initialize the structure using `&ir::Component` and `&ir::LibrarySignatures`.
    /// This function is called at the very beginning of the traversal
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
    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id>;

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
    /// This function let's you add custom conflicts to the graph
    /// before graph coloring is performed.
    fn custom_conflicts<F>(&self, _comp: &ir::Component, _add_conflicts: F)
    where
        F: FnMut(Vec<ir::Id>),
    {
    }

    /// Set the list of rewrites.
    fn set_rewrites(&mut self, rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>);

    /// Get the list of rewrites.
    fn get_rewrites(&self) -> &[(RRC<ir::Cell>, RRC<ir::Cell>)];
}

impl<T: ShareComponents> Visitor for T {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        self.initialize(&comp, &sigs);

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
                .and_modify(|v| v.push(cell.clone_name()))
                .or_insert_with(|| vec![cell.clone_name()]);
        }

        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());
        let group_conflicts = par_conflicts
            .all_conflicts()
            .into_grouping_map_by(|(g1, _)| g1.clone())
            .fold(
                HashMap::new(),
                |mut acc: HashMap<ir::CellType, Vec<ir::Id>>,
                 _,
                 (_, conflicted_group)| {
                    for conflict in
                        self.lookup_group_conflicts(&conflicted_group)
                    {
                        acc.entry(id_to_type[&conflict].clone())
                            .and_modify(|v| v.push(conflict.clone()))
                            .or_insert_with(|| vec![conflict]);
                    }
                    acc
                },
            );

        group_conflicts
            .into_iter()
            .for_each(|(group, conflict_group_b)| {
                for a in self.lookup_group_conflicts(&group) {
                    let g = graphs_by_type.get_mut(&id_to_type[&a]).unwrap();
                    if let Some(confs) = conflict_group_b.get(&id_to_type[&a]) {
                        for b in confs {
                            if a != b {
                                g.insert_conflict(&a, &b);
                            }
                        }
                    }
                }
            });

        // add custom conflicts
        self.custom_conflicts(&comp, |confs: Vec<ir::Id>| {
            for (a, b) in confs.iter().tuple_combinations() {
                if id_to_type[a] == id_to_type[b] {
                    if let Some(g) = graphs_by_type.get_mut(&id_to_type[a]) {
                        g.insert_conflict(a, b)
                    }
                }
            }
        });

        let mut coloring = Vec::new();
        for graph in graphs_by_type.values() {
            if graph.has_nodes() {
                coloring.extend(graph.color_greedy().iter().map(|(a, b)| {
                    (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
                }));
            }
        }

        // apply the coloring as a renaming of registers for both groups
        // and continuous assignments
        let builder = ir::Builder::new(comp, sigs);
        for group_ref in builder.component.groups.iter() {
            let mut group = group_ref.borrow_mut();
            let mut assigns: Vec<_> = group.assignments.drain(..).collect();
            builder.rename_port_uses(&coloring, &mut assigns);
            group.assignments = assigns;
        }

        let mut assigns: Vec<_> =
            builder.component.continuous_assignments.drain(..).collect();
        builder.rename_port_uses(&coloring, &mut assigns);
        builder.component.continuous_assignments = assigns;

        self.set_rewrites(coloring);

        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;

        // XXX(sam), is just having a single cell -> cell map for
        // rewrites sufficient. or do you need cell, group_id -> cell

        let parent = if let ir::PortParent::Cell(cell_wref) =
            &cond_port.borrow().parent
        {
            Some(cell_wref.upgrade())
        } else {
            None
        };

        if let Some(cell) = parent {
            // find rewrite for conditional port cell
            let rewrite = self
                .get_rewrites()
                .iter()
                .find(|(c, _)| Rc::ptr_eq(c, &cell));

            if let Some((_, new_cell)) = rewrite {
                let new_port = new_cell.borrow().get(&cond_port.borrow().name);
                s.port = new_port;
            }
        };

        Ok(Action::Continue)
    }

    // Rewrite the name of the cond port if this group was re-written.
    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;
        // Check if the cell associated with the port was rewritten for the cond
        // group.
        let parent = if let ir::PortParent::Cell(cell_wref) =
            &cond_port.borrow().parent
        {
            Some(cell_wref.upgrade())
        } else {
            None
        };

        if let Some(cell) = parent {
            // find rewrite for conditional port cell
            let rewrite = self
                .get_rewrites()
                .iter()
                .find(|(c, _)| Rc::ptr_eq(c, &cell));

            if let Some((_, new_cell)) = rewrite {
                let new_port = new_cell.borrow().get(&cond_port.borrow().name);
                s.port = new_port;
            }
        };

        Ok(Action::Continue)
    }

    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) -> VisResult {
        // rename inputs
        for (_id, src) in &s.inputs {
            let parent =
                if let ir::PortParent::Cell(cell_wref) = &src.borrow().parent {
                    Some(cell_wref.upgrade())
                } else {
                    None
                };

            if let Some(cell) = parent {
                // find rewrite for conditional port cell
                let rewrite = self
                    .get_rewrites()
                    .iter()
                    .find(|(c, _)| Rc::ptr_eq(c, &cell));

                if let Some((_, new_cell)) = rewrite {
                    let new_port = new_cell.borrow().get(&src.borrow().name);
                    *src.borrow_mut() = new_port.borrow().clone();
                }
            };
        }

        // rename outputs
        for (_id, dest) in &s.outputs {
            let parent = if let ir::PortParent::Cell(cell_wref) =
                &dest.borrow().parent
            {
                Some(cell_wref.upgrade())
            } else {
                None
            };

            if let Some(cell) = parent {
                // find rewrite for conditional port cell
                let rewrite = self
                    .get_rewrites()
                    .iter()
                    .find(|(c, _)| Rc::ptr_eq(c, &cell));

                if let Some((_, new_cell)) = rewrite {
                    let new_port = new_cell.borrow().get(&dest.borrow().name);
                    *dest.borrow_mut() = new_port.borrow().clone();
                }
            };
        }

        Ok(Action::Continue)
    }
}
