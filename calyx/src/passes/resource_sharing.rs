use crate::analysis;
use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
    RRC,
};
use ir::traversal::{Action, VisResult};
use itertools::Itertools;
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

        println!("{}", conflicts.to_string());

        // Map from group name to the cells its been assigned.
        let mut cell_assigns: HashMap<ir::Id, Vec<RRC<ir::Cell>>> =
            HashMap::new();

        // Sort groups in-order of conflict degree.
        let sorted: Vec<_> = comp
            .groups
            .iter()
            .sorted_by(|g1, g2| {
                // XXX(rachit): Potential performance pitfall since
                // all_conflicts iterates over the conflicts graph.
                conflicts
                    .all_conflicts(g2)
                    .len()
                    .cmp(&conflicts.all_conflicts(g1).len())
            })
            .collect();

        for group in sorted {
            // Find all the primitives already used by neighbours.
            let all_conflicts = conflicts
                .all_conflicts(group)
                .into_iter()
                .flat_map(|g| cell_assigns.get(&g.borrow().name))
                .flatten()
                .collect::<Vec<_>>();

            // New assignments for this cell.
            let mut assigns: Vec<RRC<ir::Cell>> = Vec::new();
            for old_cell in
                analysis::ReadWriteSet::uses(&group.borrow().assignments)
            {
                // If this is a primitive cell
                if let ir::CellType::Primitive { name: prim, .. } =
                    &old_cell.borrow().prototype
                {
                    // Find a cell of this primitive type that hasn't been used
                    // by the neighbours.
                    let cell = cell_map[&prim]
                        .iter()
                        .find(|c| {
                            !all_conflicts.iter().any(|uc| Rc::ptr_eq(uc, c))
                        })
                        .expect("Failed to find a non-conflicting cell.");

                    println!(
                        "{}: {}",
                        old_cell.borrow().name,
                        cell.borrow().name
                    );

                    // XXX: Apply this rewrite on the group.

                    // Save the performed assignment to `cell_assigns`.
                    assigns.push(Rc::clone(cell));
                }
            }
            cell_assigns.insert(group.borrow().name.clone(), assigns);
            println!("==============");
        }

        Ok(Action::Stop)
    }
}
