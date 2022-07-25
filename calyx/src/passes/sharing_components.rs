//! Defines common traits for methods that attempt to share components.
use crate::{analysis::GraphColoring, ir};
use ir::{
    traversal::{Action, VisResult, Visitor},
    CloneName, RRC,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

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
        let start = Instant::now();
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

        /*// get all of the invokes and enables.
        let mut invokes_enables = HashSet::new();
        get_invokes_enables(&comp.control.borrow(), &mut invokes_enables);

        // Maps node to map. node is an invoke/enable. map holds
        // the name of all the cells live at node, organized by
        // cell type. All nodes should be accounted for in this map.
        // Ex: if std_reg(32) r and std_add(32) a are alive at group G,
        // then the map would have entry: G: {std_reg(32): r, std_add(32): a}
        let mut node_by_type_map: HashMap<
            ir::Id,
            HashMap<&ir::CellType, HashSet<&ir::Id>>,
        > = HashMap::new();

        // Build node_by_type_map.
        for node in &invokes_enables {
            let node_conflicts = self.lookup_node_conflicts(node);
            if node_conflicts.is_empty() {
                // If node has no live cells, add an empty Map as its entry,
                // since we want to have all invokes/enables accounted for.
                node_by_type_map.insert(node.clone(), HashMap::new());
            } else {
                let live_at_node =
                    node_by_type_map.entry(node.clone()).or_default();
                for conflict in node_conflicts {
                    live_at_node
                        .entry(&id_to_type[conflict])
                        .or_default()
                        .insert(conflict);
                }
            }
        }

        // Closure so that we can take a group/invoke, and get all of the cells live
        // at that group/invoke, *organized by type* in the form of a HashMap.
        let lookup_conflicts_by_type =
            |node: &ir::Id| -> &HashMap<&ir::CellType, HashSet<&ir::Id>> {
                node_by_type_map.get(node).unwrap_or_else(|| {
                    unreachable!("no node conflict map for {}", node)
                })
            };

        // conflict (a,b) is in par_conflicts if a and b run in parallel w/ each other
        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        // Building node_conflicts,which is a map from nodes to another map.
        // nodes are inovkes/enables. maps are the cells live at nodes that may be run in
        // parrallel with node, and these cells are again organized by cell type.
        let mut node_conflicts = par_conflicts
            .all_conflicts()
            .into_grouping_map_by(|(g1, _)| g1.clone())
            .fold(
                HashMap::<&ir::CellType, HashSet<&ir::Id>>::new(),
                |mut acc, _, (_, conflicted_group)| {
                    let new_conflicts =
                        lookup_conflicts_by_type(&conflicted_group);
                    for (cell_type, nodes) in new_conflicts {
                        acc.entry(cell_type).or_default().extend(nodes);
                    }
                    acc
                },
            );

        // add conflicts
        for node_name in &invokes_enables {
            let node_confs_by_type = lookup_conflicts_by_type(node_name);
            match node_conflicts.get_mut(node_name) {
                None => {
                    // There are no nodes running in parallel to node_name. In
                    // this case, all we have to do is add conflicts within node_name
                    for (cell_type, confs) in node_confs_by_type {
                        let g = graphs_by_type.get_mut(cell_type).unwrap();
                        // notice how we only perform tuple_combinations on cells that we
                        // know are the same type. This is faster than creating
                        // tuple_combinations, and then checking whether they're the
                        // same type.
                        for (a, b) in confs.iter().tuple_combinations() {
                            g.insert_conflict(a, b)
                        }
                    }
                }
                Some(conflict_map) => {
                    // There are some cells that are live in parallel to node_name
                    for (cell_type, a_confs) in node_confs_by_type {
                        let g = graphs_by_type.get_mut(cell_type).unwrap();
                        if let Some(b_confs) = conflict_map.get_mut(cell_type) {
                            // Since we know a and b are the same type, we can add conflicts w/o
                            // checking type.
                            for &a in a_confs {
                                for &b in b_confs.iter() {
                                    if a != b {
                                        g.insert_conflict(a, b);
                                    }
                                }
                                // so that there are conflicts between each cell
                                // in a_confs. We do this instead of doing
                                // tuple_combinations() on a_confs.
                                b_confs.insert(a);
                            }
                        } else {
                            // If there are no cells of type cell_type that coudl be run in parallel
                            // with node_name, then all we have to do is add conflicts
                            // within a_confs
                            for (a, b) in a_confs.iter().tuple_combinations() {
                                g.insert_conflict(a, b)
                            }
                        }
                    }
                }
            }
        }*/

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

/*//Gets the names of all the cells invoked (using an invoke control statement)
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
}*/
