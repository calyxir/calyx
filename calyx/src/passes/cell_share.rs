use crate::errors::CalyxResult;
use crate::{
    analysis::{GraphColoring, LiveRangeAnalysis, ReadWriteSet, ShareSet},
    ir::{
        self,
        traversal::Named,
        traversal::{Action, ConstructVisitor, VisResult, Visitor},
        CloneName,
    },
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Given a [LiveRangeAnalysis] that specifies the "share" and "state_share" cells
/// alive at each group, minimizes the cells used for each component.
///
/// This works by constructing an interference graph for each alive "state_share" cell.
/// If two cells are ever alive at the same time, then there is an edge
/// between them in the interference graph. Additionally, if two cells
/// are different prototypes, then there is an edge between them.
///
/// A greedy graph coloring algorithm on the interference graph
/// is used to assign each cell a name.
///
/// This pass only renames uses of cells. [crate::passes::DeadCellRemoval] should be run after this
/// to actually remove the definitions.
pub struct CellShare {
    live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: ShareSet,

    /// Set of shareable components (as type names)
    shareable: ShareSet,

    /// Cell active in continuous assignments, or ref cells (we want to ignore both)
    cont_ref_cells: HashSet<ir::Id>,
}

impl Named for CellShare {
    fn name() -> &'static str {
        "cell-share"
    }
    fn description() -> &'static str {
        "use the fewest possible cells"
    }
}

impl ConstructVisitor for CellShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let state_shareable = ShareSet::from_context::<true>(ctx);
        let shareable = ShareSet::from_context::<false>(ctx);

        Ok(CellShare {
            live: LiveRangeAnalysis::default(),
            rewrites: HashMap::new(),
            cont_ref_cells: HashSet::new(),
            state_shareable,
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.cont_ref_cells = HashSet::new();
    }
}

impl CellShare {
    pub fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        //add cont cells
        self.cont_ref_cells =
            ReadWriteSet::uses(comp.continuous_assignments.iter())
                .map(|cr| cr.borrow().clone_name())
                .collect();
        //add ref cells
        self.cont_ref_cells.extend(
            comp.cells
                .iter()
                .filter(|cell| cell.borrow().is_reference())
                .map(|cell| cell.borrow().clone_name()),
        );

        // TODO(rachit): Pass cont_ref_cells to LiveRangeAnalysis so that it ignores unneccessary
        // cells.
        self.live = LiveRangeAnalysis::new(
            &mut *comp.control.borrow_mut(),
            self.state_shareable.clone(),
            self.shareable.clone(),
        );
    }

    pub fn cell_filter(&self, cell: &ir::Cell) -> bool {
        // Cells used in continuous assignments cannot be shared, nor can ref cells.
        if self.cont_ref_cells.contains(cell.name()) {
            return false;
        }
        if let Some(name) = cell.type_name() {
            self.state_shareable.contains(name) || self.shareable.contains(name)
        } else {
            false
        }
    }
}

/// The algorithm that runs is:
///  - instantiate conflict graph using all component cells that satisfy `cell_filter`
///  - use [ScheduleConflicts] to find groups/invokes that run in parallel with each other
///  - for each tuple combination of cells that return true on cell_filter(), c1 and c2
///  - first determine if their live ranges overlap. If so, then insert a conflict between
///  c1 and c2
///  - if c1 and c2 don't have overlapping live ranges, check if c1 and c2 are ever
///  live at within the same par block, and they are live at different children
///  of the par block, then add a conflict.
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

        // We assume unique ids have already been computed by LiveRangeAnalysis

        // live_once_map maps celltypes to maps that map cells to control statements
        // in which the cell was live for at least one group/invoke. Furthermore,
        // only control statements that are direct children of par blocks
        // are included in this map.
        let mut live_once_map = HashMap::new();
        // Maps every control statement that is a direct child of a par block to
        // its parent par block. (maps id number to id number)
        let mut par_thread_map = HashMap::new();
        // Maps celltype to map that maps cells to groups/invokes in which the cell is live.
        let mut live_cell_map = HashMap::new();
        // build live_once_map and par_thread_map
        self.live.get_live_control_data(
            &mut live_once_map,
            &mut par_thread_map,
            &mut live_cell_map,
            &HashSet::new(),
            &*comp.control.borrow(),
        );

        // Adding the conflicts
        for (cell_type, cells) in &cells_by_type {
            // Run remove_dead_cells before this cell-share pass.
            let g = graphs_by_type.get_mut(cell_type).unwrap();

            // mapping from cell names to the enables/invokes in which it is live
            let cell_to_nodes =
                live_cell_map.entry(cell_type.clone()).or_default();
            // mapping of cell names to the control statements in which it is live
            // at least once. Only control statements that are direct children of
            // par blocks are included
            let cell_to_control =
                live_once_map.entry(cell_type.clone()).or_default();
            for (a, b) in cells.iter().tuple_combinations() {
                // checking if live ranges overlap
                // nodes (groups/invokes) in which a is live
                if let Some(live_a) = cell_to_nodes.get(a) {
                    if let Some(live_b) = cell_to_nodes.get(b) {
                        if !live_a.is_disjoint(live_b) {
                            g.insert_conflict(a, b);
                            continue;
                        }
                    }
                }
                // checking if b is live at any groups/invokes running in parallel
                // to groups/invokes live at a
                // get the children of pars in which a was alive "at least once"
                if let Some(live_once_a) = cell_to_control.get(a) {
                    // get the children of pars in which b was alive "at least once"
                    if let Some(live_once_b) = cell_to_control.get(b) {
                        'outer: for live_a in live_once_a {
                            for live_b in live_once_b {
                                // a and b are live within the same par block but not within
                                // the same child thread, then insert conflict.
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
