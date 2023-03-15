use itertools::Itertools;

use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};
use std::collections::{HashMap, HashSet};

// maps combinational combinational components to set of all combinational components that it reads from
// so the entries are (comb comp, <set of comb components that write to comb comp>)
fn get_comb_dependence_map(
    assigns: &Vec<ir::Assignment>,
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
fn get_non_comb_writes(assigns: &Vec<ir::Assignment>) -> Vec<ir::Id> {
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

impl Visitor for DeadAssignmentRemoval {
    fn enable(
        &mut self,
        s: &mut ir::Enable,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut comb_dependence_map: HashMap<ir::Id, HashSet<ir::Id>> =
            get_comb_dependence_map(&s.group.borrow().assignments);
        let mut non_comb_writes: Vec<ir::Id> =
            get_non_comb_writes(&s.group.borrow().assignments);
        // To be a used_comb, must
        // a) be a non_comb_write
        // b) writes to a used_comb
        let mut used_combs: HashSet<ir::Id> = HashSet::new();
        // while loop is bound by size of comb_dependence_map, which is bound
        // in size by number of ports in the group's assignments
        while !(non_comb_writes.is_empty()) {
            let used = non_comb_writes.pop().unwrap();
            // add all writes to used to non_comb_writes
            if let Some(write_to_used) = comb_dependence_map.remove(&used) {
                for write in write_to_used {
                    non_comb_writes.push(write);
                }
            }
            // add used to used_combs
            used_combs.insert(used);
        }

        let used_assigns: Vec<ir::Assignment> = s
            .group
            .borrow_mut()
            .assignments
            .drain(..)
            .filter(|assign| {
                let dst = assign.dst.borrow();
                // if dst is a combinational component, must be used
                if dst.parent_is_comb() {
                    return used_combs.contains(&dst.get_parent_name());
                }
                true
            })
            .collect_vec();
        s.group.borrow_mut().assignments = used_assigns;
        Ok(Action::Continue)
    }
}
