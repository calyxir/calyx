use super::sharing_components::ShareComponents;
use crate::{
    analysis::{GraphColoring, LiveRangeAnalysis},
    ir::{self, traversal::Named},
};

/// Given a `LiveRangeAnalysis` that specifies the registers alive at each
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
/// This pass only renames uses of registers. `DeadCellRemoval` should be run after this
/// to actually remove the register definitions.
#[derive(Default)]
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
    rewrites: Vec<(ir::RRC<ir::Cell>, ir::RRC<ir::Cell>)>,
}

impl Named for MinimizeRegs {
    fn name() -> &'static str {
        "minimize-regs"
    }
    fn description() -> &'static str {
        "use the fewest possible registers"
    }
}

impl ShareComponents for MinimizeRegs {
    fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        self.live = LiveRangeAnalysis::new(&comp, &*comp.control.borrow());
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.live.get(group_name).into_iter().cloned().collect()
    }

    fn cell_filter(
        &self,
        cell: &ir::Cell,
        _sigs: &ir::LibrarySignatures,
    ) -> bool {
        if let Some(name) = cell.type_name() {
            name == "std_reg"
        } else {
            false
        }
    }

    fn custom_conflicts(
        &self,
        comp: &ir::Component,
        graph: &mut GraphColoring<ir::Id>,
    ) {
        for group in &comp.groups {
            let conflicts = self.live.get(&group.borrow().name);
            graph.insert_conflicts(conflicts.iter());
        }
    }

    fn set_rewrites(
        &mut self,
        rewrites: Vec<(ir::RRC<ir::Cell>, ir::RRC<ir::Cell>)>,
    ) {
        self.rewrites = rewrites;
    }

    fn get_rewrites<'a>(
        &'a self,
    ) -> &'a [(ir::RRC<ir::Cell>, ir::RRC<ir::Cell>)] {
        &self.rewrites
    }
}
