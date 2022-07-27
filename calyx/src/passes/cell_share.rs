use crate::analysis::GraphColoring;
use crate::errors::CalyxResult;
use crate::{
    analysis::{LiveRangeAnalysis, ReadWriteSet, ShareSet},
    ir::{self, traversal::ConstructVisitor, traversal::Named, CloneName},
};
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
    pub live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: ShareSet,

    /// Set of shareable components (as type names)
    shareable: ShareSet,

    /// Cell active in continuous assignments, or ref cells (we want to ignore both)
    cont_ref_cells: HashSet<ir::Id>,

    id_to_type: HashMap<ir::Id, ir::CellType>,

    rev_map: HashMap<ir::CellType, HashMap<ir::Id, HashSet<ir::Id>>>,

    par_conflicts_map: HashMap<ir::Id, Vec<ir::Id>>,
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
            id_to_type: HashMap::new(),
            rev_map: HashMap::new(),
            par_conflicts_map: HashMap::new(),
            state_shareable,
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.cont_ref_cells = HashSet::new();
        self.id_to_type = HashMap::new();
        self.rev_map = HashMap::new();
        self.par_conflicts_map = HashMap::new();
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
            comp,
            &*comp.control.borrow(),
            self.state_shareable.clone(),
            self.shareable.clone(),
        );
    }

    pub fn lookup_node_conflicts(
        &self,
        node_name: &ir::Id,
    ) -> HashMap<&ir::CellType, HashSet<&ir::Id>> {
        self.live
            .get(node_name)
            .iter()
            .map(|(cell_type, cell_names)| {
                (
                    cell_type,
                    cell_names
                        .iter()
                        .filter(|cell_name| {
                            !self.cont_ref_cells.contains(cell_name)
                        })
                        .collect(),
                )
            })
            // TODO(rachit): Once we make the above change and LiveRangeAnalysis ignores
            // cont_ref_cells during construction, we do not need this filter call.
            .collect()
    }

    pub fn cell_filter(&self, cell: &ir::Cell) -> bool {
        // Cells used in continuous assignments cannot be shared.
        if self.cont_ref_cells.contains(cell.name()) {
            return false;
        }
        if let Some(name) = cell.type_name() {
            self.state_shareable.contains(name) || self.shareable.contains(name)
        } else {
            false
        }
    }

    pub fn set_id_to_type(
        &mut self,
        id_to_type: HashMap<ir::Id, ir::CellType>,
    ) {
        self.id_to_type = id_to_type;
    }

    pub fn set_rev_map(
        &mut self,
        rev_map: HashMap<ir::CellType, HashMap<ir::Id, HashSet<ir::Id>>>,
    ) {
        self.rev_map = rev_map;
    }

    pub fn set_par_conflicts_map(
        &mut self,
        par_conflicts_map: HashMap<ir::Id, Vec<ir::Id>>,
    ) {
        self.par_conflicts_map = par_conflicts_map;
    }

    pub fn add_conflicts(
        &self,
        graphs_by_type: &mut HashMap<ir::CellType, GraphColoring<ir::Id>>,
        c: &ir::Control,
        is_in_par: bool,
    ) -> HashMap<&ir::CellType, HashSet<&ir::Id>> {
        match c {
            ir::Control::Empty(_) => HashMap::new(),
            ir::Control::Invoke(ir::Invoke { comp, .. }) => {
                self.lookup_node_conflicts(&comp.clone_name())
            }
            ir::Control::Enable(ir::Enable { group, .. }) => {
                self.lookup_node_conflicts(&group.clone_name())
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                let mut acc: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                for stmt in stmts {
                    let new_confs =
                        self.add_conflicts(graphs_by_type, stmt, is_in_par);
                    if is_in_par {
                        for (cell_type, nodes) in new_confs {
                            acc.entry(cell_type).or_default().extend(nodes);
                        }
                    }
                }
                acc
            }
            // *Assuming we have removed comb groups*
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                let mut tbranch_confs =
                    self.add_conflicts(graphs_by_type, &*tbranch, is_in_par);
                let fbranch_confs =
                    self.add_conflicts(graphs_by_type, &*fbranch, is_in_par);
                if is_in_par {
                    for (cell_type, nodes) in fbranch_confs {
                        tbranch_confs
                            .entry(cell_type)
                            .or_default()
                            .extend(nodes);
                    }
                }
                tbranch_confs
            }
            ir::Control::While(ir::While { body, .. }) => {
                self.add_conflicts(graphs_by_type, &*body, is_in_par)
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let mut acc: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                // if stmts = {1;2;3;4}, then we want to add conflicts between
                // 1-2, 1-3, 1-4, 2-3, 2-4, and 3-4. We calculate these conflicts
                // while creating acc.
                for stmt in stmts {
                    let new_confs =
                        self.add_conflicts(graphs_by_type, stmt, true);
                    for (cell_type, live_cells) in new_confs {
                        let g = graphs_by_type.get_mut(cell_type).unwrap();
                        if let Some(conflicting_cells) = acc.get_mut(cell_type)
                        {
                            for cell in live_cells.iter() {
                                for conflict in conflicting_cells.iter() {
                                    if cell != conflict {
                                        g.insert_conflict(cell, conflict);
                                    }
                                }
                            }
                            // so that cells in the next stmts will conflict w/
                            // live_cells plus all of the cells we've already
                            // iterated over
                            conflicting_cells.extend(live_cells);
                        } else {
                            acc.insert(cell_type, live_cells);
                        }
                    }
                }
                acc
            }
        }
    }
}
