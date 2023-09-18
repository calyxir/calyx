use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use std::collections::{HashMap, HashSet};

// Construct map from combinational instances to all the combinational instances that write to them.
// So the entries are (comb comp, <set of comb components that write to comb comp>)
fn get_comb_dependence_map<T>(
    assigns: &Vec<ir::Assignment<T>>,
) -> HashMap<ir::Id, HashSet<ir::Id>> {
    let mut comb_dependence_map: HashMap<ir::Id, HashSet<ir::Id>> =
        HashMap::new();
    for assign in assigns {
        let src = assign.src.borrow();
        let dst = assign.dst.borrow();
        let dst_name = dst.get_parent_name();
        if dst.parent_is_comb() {
            if src.parent_is_comb() {
                comb_dependence_map
                    .entry(dst_name)
                    .or_default()
                    .insert(src.get_parent_name());
            }
            for p in assign.guard.all_ports() {
                if p.borrow().parent_is_comb() {
                    comb_dependence_map
                        .entry(dst_name)
                        .or_default()
                        .insert(p.borrow().get_parent_name());
                }
            }
        }
    }
    comb_dependence_map
}

// non_comb_writes includes all combinational cells that write to
// something besides a combinational cell
// i.e., the combinational cells that write to group holes or stateful cells
fn get_non_comb_writes<T>(assigns: &Vec<ir::Assignment<T>>) -> Vec<ir::Id> {
    let mut non_comb_writes: Vec<ir::Id> = Vec::new();
    for assign in assigns {
        if !assign.dst.borrow().parent_is_comb() {
            let src = assign.src.borrow();
            let src_name = src.get_parent_name();
            if src.parent_is_comb() {
                non_comb_writes.push(src_name);
            }
            for p in assign.guard.all_ports() {
                let p_name = p.borrow().get_parent_name();
                if p.borrow().parent_is_comb() {
                    non_comb_writes.push(p_name);
                }
            }
        }
    }
    non_comb_writes
}

/// Removes unused assigns from groups.
/// Analyzes the writes to combinational cells in groups
/// In order for a combinational cell to be considered "used", it must:
/// 1) write to a non-combinational cell/group hole
/// 2) write to a non-combinational cell that has been shown to be "used"
#[derive(Default)]
pub struct DeadAssignmentRemoval;

impl Named for DeadAssignmentRemoval {
    fn name() -> &'static str {
        "dead-assign-removal"
    }

    fn description() -> &'static str {
        "removes assignments that are never used inside a group"
    }
}

/// Saturate the combinational dependence map by repeatedly adding used cells till we reach a fixed point.
fn saturate_dep_maps(
    mut comb_dep_map: HashMap<ir::Id, HashSet<ir::Id>>,
    mut non_comb_writes: Vec<ir::Id>,
) -> HashSet<ir::Id> {
    // To be a used_comb, must
    // a) be a non_comb_write
    // b) writes to a used_comb
    let mut used_combs = HashSet::new();
    // while loop is bound by size of comb_dependence_map, which is bound
    // in size by number of ports in the group's assignments
    while let Some(used) = non_comb_writes.pop() {
        // add all writes to used to non_comb_writes
        if let Some(write_to_used) = comb_dep_map.remove(&used) {
            for write in write_to_used {
                non_comb_writes.push(write);
            }
        }
        // add used to used_combs
        used_combs.insert(used);
    }

    used_combs
}

impl Visitor for DeadAssignmentRemoval {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let cont_comb_dep_map =
            get_comb_dependence_map(&comp.continuous_assignments);
        let cont_non_comb_writes =
            get_non_comb_writes(&comp.continuous_assignments);

        for gr in comp.groups.iter() {
            let group = gr.borrow();

            // Construct the dependence maps from the group assignments and extend using the continuous assignments
            let mut comb_dependence_map =
                get_comb_dependence_map(&group.assignments);
            comb_dependence_map.extend(cont_comb_dep_map.clone());

            let mut non_comb_writes = get_non_comb_writes(&group.assignments);
            non_comb_writes.extend(cont_non_comb_writes.clone());

            let used_combs =
                saturate_dep_maps(comb_dependence_map, non_comb_writes);

            // Explicit drop so we don't get already borrowed error from mutable borrow.
            drop(group);

            gr.borrow_mut().assignments.retain(|assign| {
                let dst = assign.dst.borrow();
                // if dst is a combinational component, must be used
                if dst.parent_is_comb() {
                    return used_combs.contains(&dst.get_parent_name());
                }
                // Make sure that the assignment's guard it not false
                !assign.guard.is_false()
            });
        }

        Ok(Action::Stop)
    }
}
