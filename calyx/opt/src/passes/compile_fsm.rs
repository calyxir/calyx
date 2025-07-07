use crate::{
    analysis::{FSMCallGraph, StatePossibility},
    traversal::{Action, ConstructVisitor, Named, VisResult, Visitor},
};
use calyx_ir::{self as ir};
use calyx_utils::CalyxResult;
pub struct CompileFSM {}

impl Named for CompileFSM {
    fn name() -> &'static str {
        "compile-fsm"
    }
    fn description() -> &'static str {
        "compiles a static repeat into an FSM construct"
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
        let mut call_graph = FSMCallGraph::new();

        println!("BEFORE");

        println!("top level pointer:");
        let mut top_level = call_graph
            .build_from_control(&comp.control.borrow())
            .unwrap();
        println!("{:?}", top_level);

        println!();
        println!("fsms:");
        call_graph.graph.iter().enumerate().for_each(|(i, afsm)| {
            println!("fsm {i}: {:?}", afsm);
        });

        println!();
        println!("AFTER");
        if let StatePossibility::Call(call) = &mut top_level {
            call.postorder_analysis();
        }
        println!("{:?}", top_level);

        Ok(Action::Continue)
    }
}
