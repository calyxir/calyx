use super::sharing_components::ShareComponents;
use crate::errors::CalyxResult;
use crate::{
    analysis::{LiveRangeAnalysis, ReadWriteSet},
    ir::{self, traversal::ConstructVisitor, traversal::Named, CloneName},
};
use std::collections::{HashMap, HashSet};

/// Given a [LiveRangeAnalysis] that specifies the registers alive at each
/// group, minimize the registers used for each component.
///
/// This works by constructing an interference graph for each alive register.
/// If two registers are ever alive at the same time, then there is an edge
/// between them in the interference graph. Additionally, if two registers
/// are different sizes, then there is an edge between them.
///
/// A greedy graph coloring algorithm on the interference graph
/// is used to assign each register a name.
///
/// This pass only renames uses of registers. [crate::passes::DeadCellRemoval] should be run after this
/// to actually remove the register definitions.
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: HashSet<ir::Id>,

    /// Set of shareable components (as type names)
    shareable: HashSet<ir::Id>,

    /// Cell active in continuous assignments
    cont_cells: HashSet<ir::Id>,
}

impl Named for MinimizeRegs {
    fn name() -> &'static str {
        "minimize-regs"
    }
    fn description() -> &'static str {
        "use the fewest possible registers"
    }
}

impl ConstructVisitor for MinimizeRegs {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut state_shareable = HashSet::new();
        let mut shareable = HashSet::new();
        // add state_share=1 primitives to the state_shareable set
        for prim in ctx.lib.signatures() {
            if let Some(&1) = prim.attributes.get("share") {
                shareable.insert(prim.name.clone());
            } else if let Some(&1) = prim.attributes.get("state_share") {
                state_shareable.insert(prim.name.clone());
            }
        }

        // add share=1 user defined components to the shareable_components set
        for comp in &ctx.components {
            if let Some(&1) = comp.attributes.get("share") {
                shareable.insert(comp.name.clone());
            }
        }

        Ok(MinimizeRegs {
            live: LiveRangeAnalysis::default(),
            rewrites: HashMap::new(),
            cont_cells: HashSet::new(),
            state_shareable,
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.cont_cells = HashSet::new();
    }
}

impl ShareComponents for MinimizeRegs {
    fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        self.cont_cells =
            ReadWriteSet::uses(comp.continuous_assignments.iter())
                .map(|cr| cr.borrow().clone_name())
                .collect();

        self.live = LiveRangeAnalysis::new(
            comp,
            &*comp.control.borrow(),
            self.state_shareable.clone(),
            self.shareable.clone(),
        );
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.live
            .get(group_name)
            .iter()
            .filter(|cell_name| !self.cont_cells.contains(cell_name))
            .cloned()
            .collect()
    }

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
        // Cells used in continuous assignments cannot be shared.
        if self.cont_cells.contains(cell.name()) {
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
        for group in comp.groups.iter() {
            let conflicts = self.live.get(group.borrow().name());
            add_conflicts(
                conflicts
                    .iter()
                    .filter(|cell_name| !self.cont_cells.contains(cell_name))
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
