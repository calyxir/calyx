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
use itertools::Itertools;

/// Minimize use of registers
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
    graph: GraphColoring<ir::Id>,
}

impl MinimizeRegs {
    pub fn new(live: LiveRangeAnalysis) -> Self {
        MinimizeRegs {
            live,
            graph: GraphColoring::default(),
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
        comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        // XXX(sam) can move this to work on definitions rather than enables
        let conflicts = self.live.get(&comp.name, &enable.group.borrow());
        self.graph
            .insert_conflicts(&conflicts.iter().cloned().collect::<Vec<_>>());

        Ok(Action::continue_default())
    }

    fn finish(
        &mut self,
        _data: (),
        comp: &mut Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        // add constraints so that registers of different sizes can't be shared
        for a_ref in &comp.cells {
            for b_ref in &comp.cells {
                let a = a_ref.borrow();
                let b = b_ref.borrow();
                let a_correct_type = a.type_name() == Some(&"std_reg".into());
                let b_correct_type = b.type_name() == Some(&"std_reg".into());
                if !(a_correct_type && b_correct_type) {
                    continue;
                }

                if a.get_paramter(&"width".into())
                    != b.get_paramter(&"width".into())
                {
                    self.graph.insert_conflict(a.name.clone(), b.name.clone());
                }
            }
        }

        let ordering = self.live.get_all(&comp.name).sorted();
        let coloring: Vec<_> = self
            .graph
            .color_greedy_with(ordering)
            .into_iter()
            .filter(|(a, b)| a != b)
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
