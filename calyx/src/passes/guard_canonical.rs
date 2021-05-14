use crate::ir::Guard;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};

/// Canonicalizes the guard expression.
#[derive(Default)]
pub struct GuardCanonical;

impl Named for GuardCanonical {
    fn name() -> &'static str {
        "guard-canonical"
    }

    fn description() -> &'static str {
        "canonicalizes the guard expression"
    }
}

fn update_assigns(assigns: Vec<ir::Assignment>) -> Vec<ir::Assignment> {
    let mut new_assign: Vec<ir::Assignment> = Vec::new();
    for assign in assigns {
        let guard = &assign.guard;
        if guard.is_port() && assign.src.borrow().is_constant(1, 1) {
            for p in guard.all_ports() {
                let mut changed_assign = assign.clone();
                changed_assign.guard = Box::new(Guard::True);
                changed_assign.src = p;
                new_assign.push(changed_assign);
            }
        } else {
            new_assign.push(assign);
        }
    }
    new_assign
}

impl Visitor for GuardCanonical {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _ctx: &LibrarySignatures,
    ) -> VisResult {
        // For each group and continuous assignments, canonicalize guard
        // statements that has constant 1 as either a source or a guard.
        // # Example
        // ```
        // a[done] = 1'd1 ? r1.done
        //   -> a[done] = r1.done
        // a[done] = r1.done ? 1'd1
        //   -> a[done] = r1.done
        // ```
        for group in &comp.groups {
            let assigns = group.borrow_mut().assignments.drain(..).collect();
            group.borrow_mut().assignments = update_assigns(assigns);
        }
        let assigns_cont = comp.continuous_assignments.drain(..).collect();
        comp.continuous_assignments = update_assigns(assigns_cont);

        Ok(Action::Stop)
    }
}
