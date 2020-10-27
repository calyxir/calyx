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

#[derive(Default)]
#[allow(clippy::type_complexity)]
/// Internal struct to store information about resource sharing.
struct Workspace {
    /// Mapping from the name of a group the cells that have been used by it.
    pub used_cells: HashMap<ir::Id, Vec<RRC<ir::Cell>>>,

    /// Mapping from the name of a group to the (old_cell, new_cell) pairs.
    /// This is used to rewrite all uses of `old_cell` with `new_cell` in the
    /// group.
    pub rewrites: HashMap<ir::Id, Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>>,
}

/// Returns the name of the primitive used to construct this cell if the
/// primitive's "share" attribute is set to 1.
fn shareable_primitive_name(
    cell: &RRC<ir::Cell>,
    sigs: &lib::LibrarySignatures,
) -> Option<(ir::Id, ir::Binding)> {
    if let ir::CellType::Primitive {
        name,
        param_binding,
    } = &cell.borrow().prototype
    {
        if let Some(&share) = sigs[&name].attributes.get("share") {
            if share == 1 {
                return Some((name.clone(), param_binding.clone()));
            }
        }
    }
    None
}

impl Visitor for ResourceSharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        sigs: &lib::LibrarySignatures,
    ) -> VisResult {
        // Mapping from the name of the primitive to all cells that use it.
        let mut cell_map: HashMap<(ir::Id, ir::Binding), Vec<RRC<ir::Cell>>> =
            HashMap::new();
        for cell in &comp.cells {
            if let Some(name_and_binding) = shareable_primitive_name(cell, sigs)
            {
                cell_map
                    .entry(name_and_binding.clone())
                    .or_default()
                    .push(Rc::clone(cell))
            }
        }

        let conflicts =
            analysis::ScheduleConflicts::from(&*comp.control.borrow());

        // Map from group name to the cells its been assigned.
        let mut workspace = Workspace::default();

        // Sort groups in descending order of number of conflicts.
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
                .flat_map(|g| workspace.used_cells.get(&g.borrow().name))
                .flatten()
                .collect::<Vec<_>>();

            // Cells used by the generated assignment for this cell.
            let mut used_cells: Vec<RRC<ir::Cell>> = Vec::new();
            // Rewrites generated for this group.
            let mut rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)> = Vec::new();

            for old_cell in
                analysis::ReadWriteSet::uses(&group.borrow().assignments)
            {
                // If this is a primitive cell
                if let Some(name_and_binding) =
                    shareable_primitive_name(&old_cell, sigs)
                {
                    // Find a cell of this primitive type that hasn't been used
                    // by the neighbours.
                    let cell = cell_map[&name_and_binding]
                        .iter()
                        .find(|c| {
                            !all_conflicts.iter().any(|uc| Rc::ptr_eq(uc, c))
                        })
                        .expect("Failed to find a non-conflicting cell.");

                    // Rewrite current cell to the new cell.
                    rewrites.push((Rc::clone(&old_cell), Rc::clone(cell)));

                    // Save the performed assignment to `cell_assigns`.
                    used_cells.push(Rc::clone(cell));
                }
            }

            // Add the used cells and the rewrites to the workspace.
            workspace
                .used_cells
                .insert(group.borrow().name.clone(), used_cells);
            workspace
                .rewrites
                .insert(group.borrow().name.clone(), rewrites);
        }

        let builder = ir::Builder::from(comp, sigs, false);

        // Apply the generated rewrites
        for group_ref in &builder.component.groups {
            let mut group = group_ref.borrow_mut();
            let mut assigns = group.assignments.drain(..).collect::<Vec<_>>();
            for (old, new) in &workspace.rewrites[&group.name] {
                // XXX(rachit): Performance pitfall.
                // ir::Builder::rename_port_uses iterates over the entire
                // assignment list every time.
                builder.rename_port_uses(
                    Rc::clone(old),
                    Rc::clone(new),
                    &mut assigns,
                );
            }
            group.assignments = assigns;
        }

        Ok(Action::Stop)
    }
}
