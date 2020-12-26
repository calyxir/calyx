use crate::ir::{
    self,
    traversal::{Named, Visitor},
    LibrarySignatures,
};
use ir::traversal::{Action, VisResult};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;

/// Merge assignments of the form:
/// ```
/// n.p = g1 ? x;
/// n.p = g2 ? x;
/// n.p = g3 ? y;
/// ```
/// into:
/// ```
/// n.p = g1 | g2 ? x;
/// n.p = g3 ? y;
/// ```
#[derive(Default)]
pub struct MergeAssign {}

impl Named for MergeAssign {
    fn name() -> &'static str {
        "merge-assign"
    }

    fn description() -> &'static str {
        "Merge assignments with the same source-destination pairs"
    }
}

fn merge_assigns(assigns: Vec<ir::Assignment>) -> Vec<ir::Assignment> {
    // A (cell, port) pair used as a key.
    type Key = (ir::Id, ir::Id);
    // Map from (dst, src) -> Assignment
    let mut map: LinkedHashMap<(Key, Key), ir::Assignment> =
        LinkedHashMap::new();

    for assign in assigns {
        let src_key = assign.src.borrow().canonical();
        let dst_key = assign.dst.borrow().canonical();
        let key = (dst_key, src_key);
        if let Some(asgn) = map.get_mut(&key) {
            *asgn.guard |= *assign.guard;
        } else {
            map.insert(key, assign);
        }
    }

    map.into_iter()
        .sorted_by(|(k1, _), (k2, _)| Ord::cmp(k1, k2))
        .map(|(_, v)| v)
        .collect::<Vec<_>>()
}

impl Visitor for MergeAssign {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        for group in &comp.groups {
            let assigns = group.borrow_mut().assignments.drain(..).collect();
            let merged = merge_assigns(assigns);
            group.borrow_mut().assignments = merged;
        }

        let cassigns = comp.continuous_assignments.drain(..).collect();
        comp.continuous_assignments = merge_assigns(cassigns);

        Ok(Action::Stop)
    }
}
