use crate::{
    analysis::StatePossibility,
    traversal::{Action, ConstructVisitor, Named, VisResult, Visitor},
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;
pub struct FSMAnnotator {}

impl Named for FSMAnnotator {
    fn name() -> &'static str {
        "fsm-annotator"
    }
    fn description() -> &'static str {
        "annotate a control program, determining how FSMs should be allocated"
    }
}
impl ConstructVisitor for FSMAnnotator {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(FSMAnnotator {})
    }
    fn clear_data(&mut self) {}
}

fn step_control(ctrl: &mut ir::Control, annotated: &StatePossibility) {}

impl Visitor for FSMAnnotator {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let ctrl_ref = comp.control.borrow();
        let mut st_poss = StatePossibility::from(&*ctrl_ref);

        println!("BEFORE");

        println!("{:?}", st_poss);

        println!();
        println!("AFTER");

        st_poss.post_order_analysis();

        println!("{:?}", st_poss);
        println!();

        Ok(Action::Continue)
    }
}
