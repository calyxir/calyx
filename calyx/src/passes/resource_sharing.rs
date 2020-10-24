use crate::analysis;
use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
    RRC,
};
use ir::traversal::{Action, VisResult};
use std::collections::HashMap;
use std::rc::Rc;

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
        // Mapping from the name of the primitive to all cells that use it.
        let mut cell_map: HashMap<ir::Id, Vec<RRC<ir::Cell>>> = HashMap::new();
        for cell in &comp.cells {
            if let ir::CellType::Primitive { name, .. } =
                &cell.borrow().prototype
            {
                cell_map
                    .entry(name.clone())
                    .or_default()
                    .push(Rc::clone(cell))
            }
        }

        let conflicts =
            analysis::ScheduleConflicts::from(&*comp.control.borrow());

        // For each group
        // For each shareable cell used by the group
        // For each cell of this type not used by any conflicting group
        // Rewrite all instances of this cell to .
        /*for group in &comp.groups {
            group.
        }*/

        /*for group in &comp.groups {
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
        }*/
        Ok(Action::Stop)
    }
}
