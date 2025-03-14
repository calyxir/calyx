use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use std::collections::{HashMap, HashSet};

/// Finds redundant uses of cell `done` ports within combinational groups associated
/// with a single invoke, and replaces them with zero.
///
/// # Example
/// ```no_run
/// wires {
///     comb group comb_group {
///       wire.in = !invoked_cell.done ? 1'd1;
///     }
/// }
/// control {
///     invoke invoked_cell[]()() with comb_group;
/// }
/// ```
/// In `comb_group` above, the use of `invoked_cell.done` is unnecessary, since
/// `comb_group` is only active during the invocation of `invoked_cell`. So, we can
/// replace the use of `invoked_cell.done` with zero.
///
/// NOTE: This is only true if `comb_group` is *only* used by the invocation of `invoked_cell`.
/// So, the pass goes through all uses of combinational groups (invoke/while/if) and checks for
/// multiple uses of the same comb group.
#[derive(Debug, Default)]
pub struct SimplifyInvokeWith {
    /// name of comb group -> name of cell being invoked
    comb_groups_to_modify: HashMap<ir::Id, ir::Id>,
    /// comb groups used in while/if blocks, or used in multiple invokes
    /// used to filter out comb groups that we *shouldn't* modify
    /// NOTE: This needs to be in a separate set because we may process whiles/ifs before invokes.
    comb_groups_used_elsewhere: HashSet<ir::Id>,
}

impl Named for SimplifyInvokeWith {
    fn name() -> &'static str {
        "simplify-invoke-with"
    }

    fn description() -> &'static str {
        "When a comb group is attached to a singular invoke, removes uses of the invoke cell's done port"
    }
}

impl Visitor for SimplifyInvokeWith {
    fn invoke(
        &mut self,
        s: &mut calyx_ir::Invoke,
        _comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        if let Some(cg) = &s.comb_group {
            let cg_name = cg.borrow().name();
            let cell_name = s.comp.borrow().name();
            if let Some(registered_cell_name) =
                self.comb_groups_to_modify.get(&cg_name)
            {
                if !(*registered_cell_name == cell_name) {
                    // there is a different invoke that is using the same comb group
                    self.comb_groups_used_elsewhere.insert(cg_name);
                }
            } else {
                // no invokes have used this comb group so far
                self.comb_groups_to_modify.insert(cg_name, cell_name);
            }
        }
        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut calyx_ir::While,
        _comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        if let Some(comb_group) = &s.cond {
            self.comb_groups_used_elsewhere
                .insert(comb_group.borrow().name());
        }
        Ok(Action::Continue)
    }

    fn start_if(
        &mut self,
        s: &mut calyx_ir::If,
        _comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        if let Some(comb_group) = &s.cond {
            self.comb_groups_used_elsewhere
                .insert(comb_group.borrow().name());
        }
        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut calyx_ir::Component,
        sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, sigs);
        let zero = builder.add_constant(0, 1);
        // first, drop any comb groups that are used in while/ifs and in multiple invokes
        for used_comb_group in &self.comb_groups_used_elsewhere {
            self.comb_groups_to_modify.remove(used_comb_group);
        }
        // modify assignments of any remaining comb groups
        for comb_group_ref in comp.comb_groups.iter() {
            let mut comb_group = comb_group_ref.borrow_mut();
            let comb_group_name = comb_group.name();
            if let Some(cell_name) =
                self.comb_groups_to_modify.get(&comb_group_name)
            {
                let mut modified_asgns =
                    std::mem::take(&mut comb_group.assignments);
                for asgn in &mut modified_asgns {
                    asgn.for_each_port(|port_ref| {
                        let mut res = None;
                        let port = port_ref.borrow();
                        if let ir::PortParent::Cell(cell_wref) = &port.parent {
                            if cell_wref.upgrade().borrow().name() == cell_name
                                && port.name == "done"
                            {
                                res = Some(zero.borrow().get("out"));
                            }
                        }
                        res
                    });
                }
                comb_group.assignments = modified_asgns;
            }
        }

        Ok(Action::Continue)
    }
}
