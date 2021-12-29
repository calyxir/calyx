use itertools::Itertools;

use crate::analysis;
use crate::ir::Guard;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};

/// For each group and continuous assignments, canonicalize guard
/// statements that has constant 1 as either a source or a guard.
///
/// # Example
/// ```
/// a[done] = 1'd1 ? r1.done -> a[done] = r1.done
/// a[done] = r1.done ? 1'd1 -> a[done] = r1.done
/// ```
#[derive(Default)]
pub struct GuardCanonical;

impl Named for GuardCanonical {
    fn name() -> &'static str {
        "guard-canonical"
    }

    fn description() -> &'static str {
        "canonicalizes guard expressions"
    }
}

fn update_assign(mut assign: ir::Assignment) -> ir::Assignment {
    if let Guard::Port(p) = &(*assign.guard) {
        // 1'd1 ? r1.done
        if p.borrow().is_constant(1, 1) {
            assign.guard = Guard::True.into()
        }
        // r1.done ? 1'd1
        else if assign.src.borrow().is_constant(1, 1) {
            assign.src = p.clone(); //rc clone
            assign.guard = Guard::True.into();
        }
    }
    assign
}

impl Visitor for GuardCanonical {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        for gr in comp.groups.iter() {
            eprintln!("{}", gr.borrow().name());
            let assigns = gr
                .borrow_mut()
                .assignments
                .drain(..)
                .map(update_assign)
                .collect_vec();
            gr.borrow_mut().assignments =
                analysis::DataflowOrder::dataflow_sort(assigns)?;
        }
        for cgr in comp.comb_groups.iter() {
            eprintln!("{}", cgr.borrow().name());
            let assigns = cgr
                .borrow_mut()
                .assignments
                .drain(..)
                .map(update_assign)
                .collect_vec();
            cgr.borrow_mut().assignments =
                analysis::DataflowOrder::dataflow_sort(assigns)?;
        }
        let cont_assigns = comp
            .continuous_assignments
            .drain(..)
            .map(update_assign)
            .collect_vec();
        comp.continuous_assignments =
            analysis::DataflowOrder::dataflow_sort(cont_assigns)?;

        Ok(Action::Stop)
    }
}
