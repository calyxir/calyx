use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;

/// Merge assignments of the form with the same (dst_port, src_port) pairs.
///
/// # Example
/// ```
/// x.in = f.out == 1 ? 2'd0;
/// x.in = f.out == 2 ? 2'd0;
/// x.in = f.out == 3 ? 2'd2;
/// y.in = f.out == 1 ? 2'd0;
/// ```
/// into:
/// ```
/// x.in = (f.out == 1) | (f.out == 2) ? 2'd0;
/// x.in = f.out == 3 ? 2'd2;
/// y.in = f.out == 1 ? 2'd0;
/// ```
#[derive(Default)]
pub struct MergeAssign {}

impl Named for MergeAssign {
    fn name() -> &'static str {
        "merge-assigns"
    }

    fn description() -> &'static str {
        "Merge assignments with the same source-destination pairs"
    }
}

fn merge_assigns<T: Eq>(
    assigns: Vec<ir::Assignment<T>>,
) -> Vec<ir::Assignment<T>> {
    // Map from (dst, src) -> Assignment
    let mut map: LinkedHashMap<
        (ir::Canonical, ir::Canonical),
        ir::Assignment<T>,
    > = LinkedHashMap::new();

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
        _comps: &[ir::Component],
    ) -> VisResult {
        for group in comp.get_groups().iter() {
            let assigns = group.borrow_mut().assignments.drain(..).collect();
            let merged = merge_assigns(assigns);
            group.borrow_mut().assignments = merged;
        }
        for comb_group in comp.comb_groups.iter() {
            let assigns =
                comb_group.borrow_mut().assignments.drain(..).collect();
            let merged = merge_assigns(assigns);
            comb_group.borrow_mut().assignments = merged;
        }
        for st_group in comp.static_groups.iter() {
            let assigns = st_group.borrow_mut().assignments.drain(..).collect();
            let merged = merge_assigns(assigns);
            st_group.borrow_mut().assignments = merged;
        }

        let cassigns = comp.continuous_assignments.drain(..).collect();
        comp.continuous_assignments = merge_assigns(cassigns);

        Ok(Action::Stop)
    }
}
