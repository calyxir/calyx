use itertools::Itertools;

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
    live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: ShareSet,

    /// Set of shareable components (as type names)
    shareable: ShareSet,

    /// Cell active in continuous assignments, or ref cells (we want to ignore both)
    cont_ref_cells: HashSet<ir::Id>,

    /// Maps cell names to cell types
    id_to_type: HashMap<ir::Id, ir::CellType>,
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
            state_shareable,
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.cont_ref_cells = HashSet::new();
        self.id_to_type = HashMap::new();
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

    fn lookup_node_conflicts(&self, node_name: &ir::Id) -> Vec<&ir::Id> {
        self.live
            .get(node_name)
            .iter()
            // TODO(rachit): Once we make the above change and LiveRangeAnalysis ignores
            // cont_ref_cells during construction, we do not need this filter call.
            .filter(|cell_name| !self.cont_ref_cells.contains(cell_name))
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

    // Recursively goes through c and adds conflicts to graphs_by_type. It adds
    // conflicts between any cells of the same type that are either a) run in
    // parallel groups/invokes or b) are in the same group/invoke.
    // It returns all of the shareable or state_shareable cells alive at any point
    // in c, if `is_in_par` is true. If it isn't then we return anything we want
    // since we know that we won't use this value.
    pub fn insert_conflicts(
        &self,
        graphs_by_type: &mut HashMap<ir::CellType, GraphColoring<ir::Id>>,
        c: &ir::Control,
        is_in_par: bool,
    ) -> HashMap<&ir::CellType, HashSet<&ir::Id>> {
        match c {
            ir::Control::Empty(_) => HashMap::new(),
            ir::Control::Invoke(ir::Invoke { comp, .. }) => {
                // Create a Hashmap that organizes cells live at this invoke by
                // cell type
                let mut live_by_type: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                let node_conflicts =
                    self.lookup_node_conflicts(&comp.clone_name());
                if !node_conflicts.is_empty() {
                    for conflict in node_conflicts {
                        live_by_type
                            .entry(&self.id_to_type[conflict])
                            .or_default()
                            .insert(conflict);
                    }
                }
                // Add conflicts between all of the same type within the invoke statement
                for (cell_type, confs) in live_by_type.iter() {
                    let g = graphs_by_type.get_mut(cell_type).unwrap();
                    for (a, b) in confs.iter().tuple_combinations() {
                        g.insert_conflict(a, b)
                    }
                }
                // Return all cells live at this invoke, organized by cell type
                live_by_type
            }
            ir::Control::Enable(ir::Enable { group, .. }) => {
                // there should be a way to combine this and the invoke pattern match
                // since they are sbasically the same.
                let mut live_by_type: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                let node_conflicts =
                    self.lookup_node_conflicts(&group.clone_name());
                if !node_conflicts.is_empty() {
                    for conflict in node_conflicts {
                        live_by_type
                            .entry(&self.id_to_type[conflict])
                            .or_default()
                            .insert(conflict);
                    }
                }
                for (cell_type, confs) in live_by_type.iter() {
                    let g = graphs_by_type.get_mut(cell_type).unwrap();
                    for (a, b) in confs.iter().tuple_combinations() {
                        g.insert_conflict(a, b)
                    }
                }
                live_by_type
            }
            ir::Control::Seq(ir::Seq { stmts, .. }) => {
                // acc to hold all of the cells that are live at any point within
                // this seq, organized by cell type
                let mut acc: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                for stmt in stmts {
                    // Still have to recursively find the invokes/enables so that
                    // the tuple_combinations can be filled in.
                    let new_confs =
                        self.insert_conflicts(graphs_by_type, stmt, is_in_par);
                    // if this seq is not wrapped in a par at some leve, then
                    // we know we won't need this acc information, so we shouldn't
                    // waste time doing these calculations
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
                    self.insert_conflicts(graphs_by_type, &*tbranch, is_in_par);
                let fbranch_confs =
                    self.insert_conflicts(graphs_by_type, &*fbranch, is_in_par);
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
                self.insert_conflicts(graphs_by_type, &*body, is_in_par)
            }
            ir::Control::Par(ir::Par { stmts, .. }) => {
                let mut acc: HashMap<&ir::CellType, HashSet<&ir::Id>> =
                    HashMap::new();
                // if stmts = {1;2;3;4}, then we want to add conflicts between
                // 1-2, 1-3, 1-4, 2-3, 2-4, and 3-4. We calculate these conflicts
                // while creating acc.
                for stmt in stmts {
                    let new_confs =
                        self.insert_conflicts(graphs_by_type, stmt, true);
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
