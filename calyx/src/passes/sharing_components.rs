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
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Instant;

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
        F: FnMut(HashSet<ir::Id>),
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
        let start = Instant::now();
        self.initialize(comp, sigs);

        let cells = comp.cells.iter().filter(|c| self.cell_filter(&c.borrow()));

        log::info!("checkpt .5: {}ms", start.elapsed().as_millis());

        let id_to_type: HashMap<ir::Id, ir::CellType> = cells
            .clone()
            .map(|cell| (cell.clone_name(), cell.borrow().prototype.clone()))
            .collect();

        // These two options produce different ordering, which affects which
        // component's name is used.

        // Mapping from type to all cells of that type.
        /*let cells_by_type: HashMap<&ir::CellType, Vec<&ir::Id>> =
        id_to_type.iter().map(|(k, v)| (v, k)).into_group_map();*/

        let mut cells_by_type: HashMap<ir::CellType, Vec<ir::Id>> =
            HashMap::new();
        for cell in cells {
            cells_by_type
                .entry(cell.borrow().prototype.clone())
                .or_default()
                .push(cell.clone_name())
        }

        log::info!("checkpt1: {}ms", start.elapsed().as_millis());

        let mut graphs_by_type: HashMap<ir::CellType, GraphColoring<ir::Id>> =
            cells_by_type
                .into_iter()
                .map(|(key, cell_names)| {
                    (key, GraphColoring::from(cell_names.into_iter()))
                })
                .collect();

        log::info!("checkpt2: {}ms", start.elapsed().as_millis());

        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        let i = par_conflicts.all_conflicts();

        log::info!("checkpt2.4: {}ms", start.elapsed().as_millis());

        let j = i.into_grouping_map_by(|(g1, _)| g1.clone());

        log::info!("checkpt2.6: {}ms", start.elapsed().as_millis());

        let mut node_live_map: HashMap<ir::Id, Vec<&ir::Id>> = HashMap::new();
        let mut node_by_type_map: HashMap<
            ir::Id,
            HashMap<&ir::CellType, BTreeSet<&ir::Id>>,
        > = HashMap::new();

        let mut invokes_enables = HashSet::new();
        get_invokes_enables(&comp.control.borrow(), &mut invokes_enables);

        for node in invokes_enables {
            let node_conflicts = self.lookup_node_conflicts(&node);
            node_live_map
                .insert(node.clone(), self.lookup_node_conflicts(&node));

            for conflict in node_conflicts {
                node_by_type_map
                    .entry(node.clone())
                    .or_default()
                    .entry(&id_to_type[conflict])
                    .or_default()
                    .insert(conflict);
            }
        }

        let lookup_conflicts = |node: &ir::Id| -> &Vec<&ir::Id> {
            node_live_map.get(&node).unwrap_or_else(|| {
                unreachable!("do not have node_live_map entry for {}", node)
            })
        };

        let lookup_conflicts_by_type =
            |node: &ir::Id| -> &HashMap<&ir::CellType, BTreeSet<&ir::Id>> {
                node_by_type_map.get(&node).unwrap_or_else(|| {
                    unreachable!("do not have node_live_map entry for {}", node)
                })
            };

        log::info!("checkpt2.8: {}ms", start.elapsed().as_millis());

        let mut node_conflicts = j.fold(
            HashMap::<&ir::CellType, BTreeSet<&ir::Id>>::new(),
            |mut acc, _, (_, conflicted_group)| {
                let new_conflicts = lookup_conflicts_by_type(&conflicted_group);
                acc.extend(
                    new_conflicts
                        .into_iter()
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
                acc
            },
        );

        log::info!("checkpt3: {}ms", start.elapsed().as_millis());

        // add custom conflicts
        self.custom_conflicts(comp, |nodes: HashSet<ir::Id>| {
            for node_name in nodes {
                let mut emtpy_map = HashMap::new();
                let conflict_map = match node_conflicts.get_mut(&node_name) {
                    None => &mut emtpy_map,
                    Some(cmap) => cmap,
                };
                for &a in lookup_conflicts(&node_name) {
                    let g = graphs_by_type.get_mut(&id_to_type[a]).unwrap();
                    if let Some(b_confs) = conflict_map.get_mut(&id_to_type[a])
                    {
                        for &b in b_confs.iter() {
                            if a != b {
                                g.insert_conflict(a, b);
                            }
                        }
                        b_confs.insert(a);
                    } else {
                        conflict_map
                            .insert(&id_to_type[a], BTreeSet::from([a]));
                    }
                }
            }
        });

        log::info!("checkpt 4+5: {}ms", start.elapsed().as_millis());

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

        log::info!("checkpt6: {}ms", start.elapsed().as_millis());

        // Rewrite assignments using the coloring generated.
        let empty_map: ir::rewriter::PortRewriteMap = HashMap::new();
        let rewriter = ir::Rewriter::new(&coloring, &empty_map);
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| rewriter.get(port));
        });

        log::info!("checkpt7: {}ms", start.elapsed().as_millis());

        // Rewrite control uses of ports
        rewriter.rewrite_control(
            &mut *comp.control.borrow_mut(),
            &HashMap::new(),
            &HashMap::new(),
        );

        log::info!("checkpt8: {}ms", start.elapsed().as_millis());

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
