use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures,
};
use crate::errors::Error;
use crate::ir::Guard;

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

fn update_assign(assigns: Vec<ir::Assignment>) -> Vec<ir::Assignment> {
    let mut new_assign : Vec<ir::Assignment> = Vec::new();
    for assign in assigns {
        let guard = &mut assign.guard.clone();
        let src = assign.src.borrow();
        if guard.is_port() && src.is_constant(1, 1) {
            let guard_ports = assign.guard.all_ports();
            for p in guard_ports {
                let mut changed_assign = assign.clone();
                changed_assign.guard = Box::new(Guard::True);
                changed_assign.src = p;
                new_assign.push(changed_assign);
            }
        }
        else {
            new_assign.push(assign.clone());
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
        // For each group, canonicalize guard statements that has constant 1 as
        // either a source or a guard.
        // # Example
        // ```
        // a[done] = 1'd1 ? r1.done
        //   -> a[done] = r1.done
        // a[done] = r1.done ? 1'd1
        //   -> a[done] = r1.done
        // ```
        for group in &comp.groups {
            let new_assign = update_assign(group.borrow_mut().assignments.clone());
            group.borrow_mut().assignments = new_assign;
        }

        Ok(Action::Stop)
    }
}