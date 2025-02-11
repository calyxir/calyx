use crate::analysis;
use crate::traversal::{Action, ConstructVisitor, Named, VisResult, Visitor};
use calyx_ir::Guard;
use calyx_ir::{self as ir, LibrarySignatures};

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
    fn from(ctx: &ir::Context) -> calyx_utils::CalyxResult<Self>
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
                // 1'd1 ? r1.done
                if p.borrow().is_constant(1, 1) {
                    assign.guard = Guard::True.into()
                }
            }
        });
        comp.for_each_static_assignment(|assign| {
            if let Guard::Port(p) = &(*assign.guard) {
                // 1'd1 ? r1.done
                if p.borrow().is_constant(1, 1) {
                    assign.guard = Guard::True.into()
                }
            }
        });

        for gr in comp.get_groups().iter() {
            // Handles group[done] = a ? 1'd1 -> group[done] = a
            let mut group = gr.borrow_mut();
            let done_assign = group.done_cond_mut();
            if let Guard::Port(p) = &(*done_assign.guard) {
                if done_assign.src.borrow().is_constant(1, 1) {
                    done_assign.src = p.clone(); //rc clone
                    done_assign.guard = Guard::True.into();
                }
            }
            // Deals with aassignment ordering
            let assigns = std::mem::take(&mut group.assignments);
            group.assignments = self.order.dataflow_sort(assigns)?;
        }
        for gr in comp.get_static_groups().iter() {
            // Do *not* handle group[done] = a ? 1'd1. Keep it as is.
            // Deals with aassignment orderin
            let mut group = gr.borrow_mut();
            let assigns = std::mem::take(&mut group.assignments);
            group.assignments = self.order.dataflow_sort(assigns)?;
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
