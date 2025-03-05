use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::{self as ir, LibrarySignatures};
use calyx_utils::CalyxResult;
use std::collections::{HashMap, HashSet};

/// Documentation
pub struct ProcessInvokeWith {
    // name of comb group -> name of cell being invoked
    comb_groups_to_modify: HashMap<ir::Id, ir::Id>,
    // comb groups seen in while/if blocks
    // this needs to be in a separate set because we may process whiles/ifs before invokes.
    comb_groups_seen_elsewhere: HashSet<ir::Id>,
}

impl Named for ProcessInvokeWith {
    fn name() -> &'static str {
        "process-invoke-with"
    }

    fn description() -> &'static str {
        "Transform `par` blocks to `seq`"
    }

    fn opts() -> Vec<crate::traversal::PassOpt> {
        vec![]
    }
}

impl ConstructVisitor for ProcessInvokeWith {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        // let opts = Self::get_opts(ctx);

        Ok(ProcessInvokeWith {
            comb_groups_to_modify: HashMap::new(),
            comb_groups_seen_elsewhere: HashSet::new(),
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Visitor for ProcessInvokeWith {
    fn invoke(
        &mut self,
        s: &mut calyx_ir::Invoke,
        _comp: &mut calyx_ir::Component,
        _sigs: &LibrarySignatures,
        _comps: &[calyx_ir::Component],
    ) -> VisResult {
        // if there is a combinational group associated with the group...
        if let Some(cg) = &s.comb_group {
            let cg_name = cg.borrow().name();
            if self.comb_groups_to_modify.contains_key(&cg_name) {
                // there is a different invoke that is using the same comb group
                self.comb_groups_seen_elsewhere.insert(cg_name);
            } else {
                // there are no previous invokes that are using this comb group
                self.comb_groups_to_modify
                    .insert(cg_name, s.comp.borrow().name());
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
            self.comb_groups_seen_elsewhere
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
            self.comb_groups_seen_elsewhere
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
        for (cg, c) in self.comb_groups_to_modify.iter() {
            println!("{}, {}", cg, c);
        }
        let mut builder = ir::Builder::new(comp, sigs);
        let one = builder.add_constant(1, 1);
        // first, drop any comb groups we've seen
        for used_comb_group in &self.comb_groups_seen_elsewhere {
            self.comb_groups_to_modify.remove(&used_comb_group);
        }
        println!("yo what the heck");
        for (cg, c) in self.comb_groups_to_modify.iter() {
            println!("{}, {}", cg, c);
        }
        // modify assignments of any remaining comb groups
        for comb_group_ref in comp.comb_groups.iter() {
            let mut comb_group = comb_group_ref.borrow_mut();
            let comb_group_name = comb_group.name();
            if self.comb_groups_to_modify.contains_key(&comb_group_name) {
                println!("{comb_group_name}");
                let cell_name =
                    self.comb_groups_to_modify.get(&comb_group_name).unwrap();
                let mut modified_asgns = comb_group.assignments.clone();
                for mut asgn in &mut modified_asgns {
                    asgn.for_each_port(|port_ref| {
                        let mut res = None;
                        let port = port_ref.borrow();
                        if let ir::PortParent::Cell(cell_wref) = &port.parent {
                            if cell_wref.upgrade().borrow().name() == cell_name
                                && port.name == "done"
                            {
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
