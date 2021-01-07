use crate::{
    analysis::{GraphColoring, LiveRangeAnalysis, ScheduleConflicts},
    ir::{
        self,
        traversal::{Named, Visitor},
        LibrarySignatures,
    },
};
use ir::{
    traversal::{Action, VisResult},
    Component,
};
use itertools::Itertools;

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
    graph: GraphColoring<ir::Id>,
    par_conflicts: ScheduleConflicts,
}

impl MinimizeRegs {
    pub fn new(live: LiveRangeAnalysis) -> Self {
        MinimizeRegs {
            live,
            graph: GraphColoring::default(),
            par_conflicts: ScheduleConflicts::default(),
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

impl Visitor for MinimizeRegs {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _s: &LibrarySignatures,
    ) -> VisResult {
        self.live = LiveRangeAnalysis::new(&comp, &*comp.control.borrow());
        self.par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        Ok(Action::Continue)
    }

    fn enable(
        &mut self,
        enable: &mut ir::Enable,
        _comp: &mut Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // add constraints between things that are alive at the same time
        let conflicts = self.live.get(&enable.group.borrow());
        self.graph
            .insert_conflicts(&conflicts.iter().cloned().collect::<Vec<_>>());

        let in_parallel: Vec<_> = self
            .par_conflicts
            .all_conflicts(&enable.group)
            .into_iter()
            .flat_map(|group| {
                self.live
                    .get(&*group.borrow())
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect();

        for conflict_here in conflicts {
            for par_conflict in &in_parallel {
                self.graph.insert_conflict(
                    conflict_here.clone(),
                    par_conflict.clone(),
                );
            }
        }

        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
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

        // used a sorted ordering to perform coloring
        let ordering = self.live.get_all().sorted();
        let coloring: Vec<_> = self
            .graph
            .color_greedy_with(ordering)
            .into_iter()
            .filter(|(a, b)| a != b)
            .map(|(a, b)| {
                (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
            })
            .collect();

        // eprintln!(
        //     "{:?}",
        //     coloring
        //         .iter()
        //         .map(|(a, b)| format!(
        //             "{} -> {}",
        //             a.borrow().name.to_string(),
        //             b.borrow().name.to_string()
        //         ))
        //         .collect::<Vec<_>>()
        // );

        // apply the coloring as a renaming of registers for both groups
        // and continuous assignments
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

        Ok(Action::Continue)
    }
}
