use super::LiveRangeAnalysis;
use crate::frontend::library::ast as lib;
use crate::{
    analysis::GraphColoring,
    ir::{
        self,
        traversal::{Named, Visitor},
    },
};
use ir::{
    traversal::{Action, VisResult},
    Component,
};

/// Minimize use of registers
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
    graph: GraphColoring<ir::Id>,
}

impl MinimizeRegs {
    pub fn new(live: LiveRangeAnalysis) -> Self {
        MinimizeRegs {
            live,
            graph: GraphColoring::new(),
        }
    }
}

impl Named for MinimizeRegs {
    fn name() -> &'static str {
        "minimize-regs"
    }
    fn description() -> &'static str {
        "use the fewest possible registers"
    }
}

impl Visitor<()> for MinimizeRegs {
    fn start_enable(
        &mut self,
        enable: &mut ir::Enable,
        _data: (),
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let name = &enable.group.borrow().name;
        eprintln!("{}", name.to_string());
        eprintln!("  {:?}", self.live.get(&enable.group.borrow()));

        let conflicts = self.live.get(&enable.group.borrow());
        self.graph.insert_conflicts(
            &conflicts.into_iter().cloned().collect::<Vec<_>>(),
        );

        Ok(Action::continue_default())
    }

    fn finish(
        &mut self,
        _data: (),
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        eprintln!("{:?}", self.live.get_all().collect::<Vec<_>>());
        let ordering = self.live.get_all();
        eprintln!("{:#?}", self.graph.color_greedy_with(ordering));
        Ok(Action::continue_default())
    }
}
