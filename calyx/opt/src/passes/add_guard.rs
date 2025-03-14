use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;

#[derive(Default)]
pub struct AddGuard;

impl Named for AddGuard {
    fn name() -> &'static str {
        "add-guard"
    }

    fn description() -> &'static str {
        "Add guard %[0: n] where n is latency of static component for each assignment in the static enable of static component."
    }
}

impl Visitor for AddGuard {
    fn start_static_control(
        &mut self,
        s: &mut ir::StaticControl,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(latency) = comp.latency {
            let lat_u64: u64 = latency.into();
            if lat_u64 > 1 {
                if let ir::StaticControl::Enable(sen) = s {
                    for assign in sen.group.borrow_mut().assignments.iter_mut()
                    {
                        let g = ir::Guard::Info(ir::StaticTiming::new((
                            0, lat_u64,
                        )));
                        let new_g = ir::Guard::And(
                            Box::new(g),
                            std::mem::replace(
                                &mut assign.guard,
                                Box::new(ir::Guard::True),
                            ),
                        );
                        assign.guard = Box::new(new_g);
                    }
                } else {
                    unreachable!(
                        "Non-Enable Static Control should have been compiled away. Run {} to do this",
                        crate::passes::StaticInliner::name()
                    );
                }
            }
        }
        Ok(Action::Continue)
    }
}
