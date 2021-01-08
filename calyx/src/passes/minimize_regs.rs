use crate::{
    analysis::{GraphColoring, LiveRangeAnalysis, ScheduleConflicts},
    ir::{
        self,
        traversal::{Named, Visitor},
        LibrarySignatures,
    },
};
use ir::traversal::{Action, VisResult};
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
pub struct MinimizeRegs;

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
        sigs: &LibrarySignatures,
    ) -> VisResult {
        let registers = comp.cells.iter().filter_map(|cell_ref| {
            let cell = cell_ref.borrow();
            if let Some(name) = cell.type_name() {
                if name == "std_reg" {
                    return Some((
                        cell.get_paramter("width").unwrap(),
                        cell.name.clone(),
                    ));
                }
            }
            None
        });

        let mut graph: GraphColoring<ir::Id> =
            GraphColoring::from(registers.clone().map(|(_, name)| name));

        let live = LiveRangeAnalysis::new(&comp, &*comp.control.borrow());
        let par_conflicts = ScheduleConflicts::from(&*comp.control.borrow());

        for group in &comp.groups {
            // Conflict edges between all things alive at the same time.
            let conflicts = live.get(&group.borrow().name);
            graph.insert_conflicts(conflicts.iter());
        }

        // Conflict edges between all groups that are enabled in parallel.
        par_conflicts
            .all_conflicts()
            .into_grouping_map_by(|(g1, _)| g1.clone())
            .fold(vec![], |mut acc, _, (_, conf_group)| {
                acc.extend(live.get(&conf_group));
                acc
            })
            .into_iter()
            .for_each(|(group, confs)| {
                let conflicts = live.get(&group);
                confs
                    .into_iter()
                    // This unique call saves a lot of time!
                    .unique()
                    .for_each(|par_conflict| {
                        for conflict_here in conflicts {
                            if conflict_here != par_conflict {
                                graph.insert_conflict(
                                    &conflict_here,
                                    &par_conflict,
                                );
                            }
                        }
                    })
            });

        // add constraints so that registers of different sizes can't be shared
        registers
            .tuple_combinations()
            .for_each(|((w1, c1), (w2, c2))| {
                if w1 != w2 {
                    graph.insert_conflict(&c1, &c2);
                }
            });

        // used a sorted ordering to perform coloring
        let ordering = live.get_all().sorted();
        let coloring: Vec<_> = graph
            .color_greedy_with(ordering)
            .into_iter()
            .filter(|(a, b)| a != b)
            .map(|(a, b)| {
                (comp.find_cell(&a).unwrap(), comp.find_cell(&b).unwrap())
            })
            .collect();

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

        Ok(Action::Stop)
    }
}
