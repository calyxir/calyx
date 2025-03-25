use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use std::collections::{HashMap, HashSet};

/// Replaces ports with constants when we can infer the value of the port.
///
/// Currently, the pass deals with a single case: If a combinational group
/// is *only* associated with invoke(s) of a single cell, then uses of the
/// cell's `go` and `done` ports in the comb group can be replaced with
/// 1 and 0 respectively.
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
/// `comb_group` is only active during the invocation of `invoked_cell`. So,
/// the pass replaces the use of `invoked_cell.done` with zero.
#[derive(Debug, Default)]
pub struct ConstantPortProp {
    /// name of comb group -> name of cell being invoked
    comb_groups_to_modify: HashMap<ir::Id, ir::Id>,
    /// comb groups used in while/if blocks, or used in multiple invokes
    /// used to filter out comb groups that we *shouldn't* modify
    /// NOTE: This needs to be in a separate set because we may process whiles/ifs before invokes.
    comb_groups_used_elsewhere: HashSet<ir::Id>,
}

impl Named for ConstantPortProp {
    fn name() -> &'static str {
        "constant-port-prop"
    }

    fn description() -> &'static str {
        "propagates constants when port values can be inferred"
    }
}

impl Visitor for ConstantPortProp {
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
                if *registered_cell_name != cell_name {
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
        let one = builder.add_constant(1, 1);
        let zero = builder.add_constant(0, 1);
        // modify assignments of comb groups that aren't used in while/ifs and in multiple invokes
        for comb_group_ref in comp.comb_groups.iter().filter(|item| {
            !self
                .comb_groups_used_elsewhere
                .contains(&item.borrow().name())
        }) {
            // for comb_group_ref in comp.comb_groups.iter() {
            let mut comb_group = comb_group_ref.borrow_mut();
            let comb_group_name = comb_group.name();
            if let Some(cell_name) =
                self.comb_groups_to_modify.get(&comb_group_name)
            {
                let mut modified_asgns =
                    std::mem::take(&mut comb_group.assignments);
                for asgn in modified_asgns.iter_mut() {
                    asgn.for_each_port(|port_ref| {
                        let mut res = None;
                        let port = port_ref.borrow();
                        if let ir::PortParent::Cell(cell_wref) = &port.parent {
                            if cell_wref.upgrade().borrow().name() == cell_name
                                && port.name == "done"
                            {
                                // replace cell.done with 0
                                res = Some(zero.borrow().get("out"));
                            } else if cell_wref.upgrade().borrow().name()
                                == cell_name
                                && port.name == "go"
                            {
                                // replace cell.done with 1
                                res = Some(one.borrow().get("out"));
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
