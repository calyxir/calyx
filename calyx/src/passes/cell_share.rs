use super::sharing_components::ShareComponents;
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

impl ShareComponents for CellShare {
    fn initialize(
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

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
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

    fn custom_conflicts<F>(&self, comp: &ir::Component, mut add_conflicts: F)
    where
        F: FnMut(Vec<ir::Id>),
    {
        let mut invokes_enables = HashSet::new();
        get_invokes_enables(&comp.control.borrow(), &mut invokes_enables);
        for invoke_enable in invokes_enables.iter() {
            let conflicts = self.live.get(invoke_enable);
            add_conflicts(
                conflicts
                    .iter()
                    .filter(|cell_name| {
                        !self.cont_ref_cells.contains(cell_name)
                    })
                    .cloned()
                    .collect(),
            );
        }
    }

    fn set_rewrites(&mut self, rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>) {
        self.rewrites = rewrites;
    }

    fn get_rewrites(&self) -> &HashMap<ir::Id, ir::RRC<ir::Cell>> {
        &self.rewrites
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
