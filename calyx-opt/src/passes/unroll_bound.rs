use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::{self as ir};

/// Fully unroll all `while` loops with a given `@bound`.
#[derive(Default)]
pub struct UnrollBounded;

impl Named for UnrollBounded {
    fn name() -> &'static str {
        "unroll-bound"
    }

    fn description() -> &'static str {
        "fully unroll loops with a given @bound"
    }
}

impl Visitor for UnrollBounded {
    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        if let Some(bound) = s.attributes.get(ir::NumAttr::Bound) {
            let body =
                *std::mem::replace(&mut s.body, Box::new(ir::Control::empty()));
            let nb = ir::Control::seq(
                (0..bound).map(|_| ir::Cloner::control(&body)).collect(),
            );
            Ok(Action::change(nb))
        } else {
            Ok(Action::Continue)
        }
    }
}
