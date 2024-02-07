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
        if comp.is_static() {
            let latency = s.get_latency();
            if let ir::StaticControl::Enable(sen) = s {
                for assign in sen.group.borrow_mut().assignments.iter_mut() {
                    let g =
                        ir::Guard::Info(ir::StaticTiming::new((0, latency)));
                    let new_g = ir::Guard::And(
                        Box::new(g),
                        std::mem::replace(
                            &mut assign.guard,
                            Box::new(ir::Guard::True),
                        ),
                    );
                    assign.guard = Box::new(new_g);
                }
            }
        }
        Ok(Action::Continue)
    }
}
