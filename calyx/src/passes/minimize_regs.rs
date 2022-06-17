use super::sharing_components::ShareComponents;
use crate::errors::CalyxResult;
use crate::{
    analysis::LiveRangeAnalysis,
    ir::{self, traversal::ConstructVisitor, traversal::Named},
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
    /// Set of shareable components (as type names)
    shareable_components: HashSet<ir::Id>,
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
        let mut shareable_components = HashSet::new();
        // add state_share=1 primitives to the shareable_components set
        for prim in ctx.lib.signatures() {
            if let Some(&1) = prim.attributes.get("state_share") {
                shareable_components.insert(prim.name.clone());
            }
        }

        Ok(MinimizeRegs {
            live: LiveRangeAnalysis::default(),
            rewrites: HashMap::new(),
            shareable_components,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
    }
}

impl ShareComponents for MinimizeRegs {
    fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        self.live = LiveRangeAnalysis::new(
            comp,
            &*comp.control.borrow(),
            self.shareable_components.clone(),
        );
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.live.get(group_name).iter().cloned().collect()
    }

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
        if let Some(name) = cell.type_name() {
            self.shareable_components.contains(name)
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
            add_conflicts(conflicts.iter().cloned().collect());
        }
    }

    fn set_rewrites(&mut self, rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>) {
        self.rewrites = rewrites;
    }

    fn get_rewrites(&self) -> &HashMap<ir::Id, ir::RRC<ir::Cell>> {
        &self.rewrites
    }
}
