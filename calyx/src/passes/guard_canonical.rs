use itertools::Itertools;

use crate::analysis;
use crate::ir::traversal::ConstructVisitor;
use crate::ir::Guard;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};

/// Perform serval canonicalizations on the program.
///
/// ## Simplifying Guards
/// For each group and continuous assignments, canonicalize guard
/// statements that has constant 1 as either a source or a guard.
///
/// # Example
/// ```
/// a[done] = 1'd1 ? r1.done -> a[done] = r1.done
/// a[done] = r1.done ? 1'd1 -> a[done] = r1.done
/// ```
///
/// ## Dataflow Ordering of Assignments
/// Uses [analysis::DataflowOrder] to sort all sets of assignments in the
/// program into dataflow order.
pub struct Canonicalize {
    // A [analysis::DataflowOrder] used to reorder assignments into dataflow order.
    order: analysis::DataflowOrder,
}

impl ConstructVisitor for Canonicalize {
    fn from(ctx: &ir::Context) -> crate::errors::CalyxResult<Self>
    where
        Self: Sized,
    {
        let read_together =
            analysis::ReadWriteSpec::read_together_specs(ctx.lib.signatures())?;
        let order = analysis::DataflowOrder::new(read_together);
        Ok(Canonicalize { order })
    }

    fn clear_data(&mut self) {
        // Data is shared between components
    }
}

impl Named for Canonicalize {
    fn name() -> &'static str {
        "canonicalize"
    }

    fn description() -> &'static str {
        "canonicalize the program"
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

impl Visitor for Canonicalize {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        for gr in comp.groups.iter() {
            let assigns = gr
                .borrow_mut()
                .assignments
                .drain(..)
                .map(update_assign)
                .collect_vec();
            gr.borrow_mut().assignments = self.order.dataflow_sort(assigns)?;
        }
        for cgr in comp.comb_groups.iter() {
            let assigns = cgr
                .borrow_mut()
                .assignments
                .drain(..)
                .map(update_assign)
                .collect_vec();
            cgr.borrow_mut().assignments = self.order.dataflow_sort(assigns)?;
        }
        let cont_assigns = comp
            .continuous_assignments
            .drain(..)
            .map(update_assign)
            .collect_vec();
        comp.continuous_assignments = self.order.dataflow_sort(cont_assigns)?;

        Ok(Action::Stop)
    }
}
