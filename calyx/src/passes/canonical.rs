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
        let order = analysis::DataflowOrder::new(ctx.lib.signatures())?;
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

impl Visitor for Canonicalize {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        comp.for_each_assignment(|assign| {
            if let Guard::Port(p) = &(*assign.guard) {
                /* Trying to cover r1.done? 1'd1.
                if p.borrow().name == "done"
                    && assign.src.borrow().clone().is_constant(1, 1)
                {
                    assign.src = p.clone();
                    assign.guard = Guard::True.into()
                }
                // 1'd1 ? r1.done
                else*/
                if p.borrow().is_constant(1, 1) {
                    assign.guard = Guard::True.into()
                }
            }
        });

        for gr in comp.groups.iter() {
            let assigns = std::mem::take(&mut gr.borrow_mut().assignments);
            gr.borrow_mut().assignments = self.order.dataflow_sort(assigns)?;
        }
        for cgr in comp.comb_groups.iter() {
            let assigns = std::mem::take(&mut cgr.borrow_mut().assignments);
            cgr.borrow_mut().assignments = self.order.dataflow_sort(assigns)?;
        }
        let cont_assigns = std::mem::take(&mut comp.continuous_assignments);
        comp.continuous_assignments = self.order.dataflow_sort(cont_assigns)?;

        Ok(Action::Stop)
    }
}
