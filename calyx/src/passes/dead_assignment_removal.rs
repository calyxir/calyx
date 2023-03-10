use itertools::Itertools;

use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use std::collections::{HashMap, HashSet};

/// Removes unused assigns from groups.
/// Analyzes the writes to combinational cells in groups
/// In order for a combinational cell to be considered "used", it must:
/// 1) write to a non-combinational cell/group hole
/// 2) write to a non-combinational cell that has been shown to be "used"
#[derive(Default)]
pub struct DeadAssignmentRemoval {}

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
        // maps combinational components to set of all combinational components that it reads from
        // so the entries are (component, <set that writes to component>)
        let mut comb_dependence_map: HashMap<ir::Id, HashSet<ir::Id>> =
            HashMap::new();
        for assign in &s.group.borrow().assignments {
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
        // To be a "used" combinational component, either:
        // 1) a non-combinational cell/group hole reads from it, or
        // 2) another "used" combinational cell reads from it
        let mut used_combs: HashSet<ir::Id> = HashSet::new();
        for assign in &s.group.borrow().assignments {
            if !assign.dst.borrow().parent_is_comb() {
                let src = assign.src.borrow();
                let src_name = src.get_parent_name();
                if src.parent_is_comb() {
                    // all writes to src are now safe, as well as src
                    match comb_dependence_map.get(&src_name) {
                        Some(writes_to_src) => used_combs.extend(writes_to_src),
                        None => (),
                    }
                    used_combs.insert(src_name);
                }
                for p in assign.guard.all_ports() {
                    let p_name = p.borrow().get_parent_name();
                    if p.borrow().parent_is_comb() {
                        // all writes to p are now safe, as well as p
                        match comb_dependence_map.get(&p_name) {
                            Some(writes_to_p) => used_combs.extend(writes_to_p),
                            None => (),
                        }
                        used_combs.insert(p_name);
                    }
                }
            }
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
                return true;
            })
            .collect_vec();
        s.group.borrow_mut().assignments = used_assigns;
        Ok(Action::Continue)
    }
}
