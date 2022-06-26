use crate::analysis::DominatorMap;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

#[derive(Default)]
/// Description goes here
pub struct InferShare {}

impl Named for InferShare {
    fn name() -> &'static str {
        "infer-share"
    }

    fn description() -> &'static str {
        "Infer User Defined Components as Shareable"
    }
}

impl Visitor for InferShare {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let map = DominatorMap::new(&mut comp.control.borrow_mut());
        println!("{:?}", map);
        Ok(Action::Continue)
    }
}
