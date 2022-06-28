use crate::analysis::DominatorMap;
use crate::errors::CalyxResult;
use crate::ir;
use crate::ir::traversal::{
    Action, ConstructVisitor, Named, VisResult, Visitor,
};

pub struct InferShare {
    print_dmap: bool,
}

impl Named for InferShare {
    fn name() -> &'static str {
        "infer-share"
    }

    fn description() -> &'static str {
        "Infer User Defined Components as Shareable"
    }
}

impl ConstructVisitor for InferShare {
    fn from(ctx: &ir::Context) -> CalyxResult<Self>
    where
        Self: Sized + Named,
    {
        let opts = Self::get_opts(&["print_dmap"], ctx);

        Ok(InferShare {
            print_dmap: opts[0],
        })
    }

    fn clear_data(&mut self) {
        /* All data can be transferred between components */
    }
}

impl Visitor for InferShare {
    fn require_postorder() -> bool {
        true
    }

    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let map = DominatorMap::new(&mut comp.control.borrow_mut());

        if self.print_dmap {
            println!("{map:?}");
        }

        Ok(Action::Continue)
    }
}
