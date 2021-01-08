use crate::analysis;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, RRC,
};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
#[allow(clippy::type_complexity)]
/// Rewrites groups to share cells marked with the "share" attribute
/// when the groups are guaranteed to never run in parallel.
pub struct ResourceSharing {
    /// Mapping from the name of a group the cells that have been used by it.
    used_cells: HashMap<ir::Id, Vec<RRC<ir::Cell>>>,

    /// Mapping from the name of a group to the (old_cell, new_cell) pairs.
    /// This is used to rewrite all uses of `old_cell` with `new_cell` in the
    /// group.
    rewrites: HashMap<ir::Id, Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>>,
}

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "shares resources between groups that don't execute in parallel"
    }
}

/// Returns the name of the primitive used to construct this cell if the
/// primitive's "share" attribute is set to 1.
fn shareable_primitive_name(
    cell: &RRC<ir::Cell>,
    sigs: &LibrarySignatures,
) -> Option<(ir::Id, ir::Binding)> {
    if let ir::CellType::Primitive {
        name,
        param_binding,
    } = &cell.borrow().prototype
    {
        if let Some(&share) = sigs.get_primitive(&name).attributes.get("share")
        {
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
        sigs: &LibrarySignatures,
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

        // Sort groups in descending order of number of conflicts.
        let sorted: Vec<_> = comp
            .groups
            .iter()
            .sorted_by(|g1, g2| {
                // XXX(rachit): Potential performance pitfall since
                // all_conflicts iterates over the conflicts graph.
                conflicts
                    .conflicts_with(&g2.borrow().name)
                    .len()
                    .cmp(&conflicts.conflicts_with(&g1.borrow().name).len())
            })
            .collect();

        for group in sorted {
            // Find all the primitives already used by neighbours.
            let all_conflicts = conflicts
                .conflicts_with(&group.borrow().name)
                .into_iter()
                .flat_map(|g| self.used_cells.get(&g))
                .cloned()
                .flatten()
                .collect::<Vec<RRC<_>>>();

            // Cells used by the generated assignment for this group.
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
                            !all_conflicts
                                .iter()
                                .chain(used_cells.iter())
                                .any(|uc| Rc::ptr_eq(uc, c))
                        })
                        .expect("Failed to find a non-conflicting cell.");

                    // Rewrite current cell to the new cell.
                    rewrites.push((Rc::clone(&old_cell), Rc::clone(cell)));

                    // Save the performed assignment to `cell_assigns`.
                    used_cells.push(Rc::clone(cell));
                }
            }

            // Add the used cells and the rewrites to the workspace.
            self.used_cells
                .insert(group.borrow().name.clone(), used_cells);
            self.rewrites.insert(group.borrow().name.clone(), rewrites);
        }

        let builder = ir::Builder::from(comp, sigs, false);

        // Apply the generated rewrites
        for group_ref in &builder.component.groups {
            let mut group = group_ref.borrow_mut();
            let mut assigns = group.assignments.drain(..).collect::<Vec<_>>();
            builder.rename_port_uses(&self.rewrites[&group.name], &mut assigns);
            group.assignments = assigns;
        }

        Ok(Action::Continue)
    }

    // Rewrite the name of the cond port if this group was re-written.
    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;
        let group_name = &s.cond.borrow().name;
        // Check if the cell associated with the port was rewritten for the cond
        // group.
        let rewrite = self.rewrites[group_name].iter().find(|(c, _)| {
            if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
            {
                return Rc::ptr_eq(c, &cell_wref.upgrade());
            }
            false
        });

        if let Some((_, new_cell)) = rewrite {
            let new_port = new_cell.borrow().get(&cond_port.borrow().name);
            s.port = new_port;
        }

        Ok(Action::Continue)
    }

    // Rewrite the name of the cond port if this group was re-written.
    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        let cond_port = &s.port;
        let group_name = &s.cond.borrow().name;
        // Check if the cell associated with the port was rewritten for the cond
        // group.
        let rewrite = self.rewrites[group_name].iter().find(|(c, _)| {
            if let ir::PortParent::Cell(cell_wref) = &cond_port.borrow().parent
            {
                return Rc::ptr_eq(c, &cell_wref.upgrade());
            }
            false
        });

        if let Some((_, new_cell)) = rewrite {
            let new_port = new_cell.borrow().get(&cond_port.borrow().name);
            s.port = new_port;
        }
        Ok(Action::Continue)
    }
}
