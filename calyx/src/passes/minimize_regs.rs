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
        let conflicts = self.live.get(&enable.group.borrow());
        self.graph.insert_conflicts(
            &conflicts.into_iter().cloned().collect::<Vec<_>>(),
        );

        Ok(Action::continue_default())
    }

    fn finish(
        &mut self,
        _data: (),
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let ordering = self.live.get_all();
        let coloring_id: Vec<(_, _)> = self
            .graph
            .color_greedy_with(ordering)
            .into_iter()
            .filter(|(a, b)| a != b)
            .collect();
        eprintln!("{:#?}", coloring_id);

        let coloring: Vec<_> = coloring_id
            .into_iter()
            .map(|(a, b)| {
                (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
            })
            .collect();

        let builder = ir::Builder::from(comp, sigs, false);

        for group_ref in &builder.component.groups {
            let mut group = group_ref.borrow_mut();
            let mut assigns: Vec<_> = group.assignments.drain(..).collect();
            builder.rename_port_uses(&coloring, &mut assigns);
            group.assignments = assigns;
        }

        let mut assigns: Vec<_> =
            builder.component.continuous_assignments.drain(..).collect();
        builder.rename_port_uses(&coloring, &mut assigns);
        builder.component.continuous_assignments = assigns;

        Ok(Action::continue_default())
    }
}
