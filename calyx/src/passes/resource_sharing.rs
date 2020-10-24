use crate::analysis::ScheduleConflicts;
use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
};
use ir::traversal::{Action, VisResult};

#[derive(Default)]
/// TODO
pub struct ResourceSharing;

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "shares resources between groups that don't execute in parallel"
    }
}

impl Visitor for ResourceSharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult {
        let conflicts = ScheduleConflicts::from(&*comp.control.borrow());
        for group in &comp.groups {
            println!(
                "{} -> {}",
                group.borrow().name,
                conflicts
                    .all_conflicts(group)
                    .into_iter()
                    .map(|g| g.borrow().name.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        }
        Ok(Action::Stop)
    }
}
