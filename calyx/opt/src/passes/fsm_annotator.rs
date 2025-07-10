use crate::{
    analysis::StatePossibility,
    traversal::{Action, ConstructVisitor, Named, VisResult, Visitor},
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;
pub struct CompileFSM {}

impl Named for CompileFSM {
    fn name() -> &'static str {
        "annotate-fsms"
    }
    fn description() -> &'static str {
        "annotate a control program, determining how FSMs should be allocated"
    }
}
impl ConstructVisitor for CompileFSM {
    fn from(_ctx: &ir::Context) -> CalyxResult<Self> {
        Ok(CompileFSM {})
    }
    fn clear_data(&mut self) {}
}

impl Visitor for CompileFSM {
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
