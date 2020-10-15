//! Guards all the non-hole assignments in a group using the group's `go` signal.
//! For example, the pass transforms this FuTIL program:
//! ```
//! group foo {
//!     x.in = cond ? 32'd1;
//!     foo[done] = reg.done;
//! }
//! ```
//! into:
//! ```
//! group foo {
//!     x.in = cond & foo[go] ? 32'd1;
//!     foo[done] = reg.done;
//! }
//! ```
use crate::frontend::library::ast as lib;
use crate::guard;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct GoInsertion {}

impl Named for GoInsertion {
    fn name() -> &'static str {
        "go-insertion"
    }

    fn description() -> &'static str {
        "removes redudant seq statements"
    }
}

impl Visitor for GoInsertion {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &lib::LibrarySignatures,
    ) -> VisResult {
        for group in &comp.groups {
            let group_go = guard!(group["go"]);
            let mut group = group.borrow_mut();
            for assign in group.assignments.iter_mut() {
                if !assign.dst.borrow().is_hole() {
                    let cur_guard = assign.guard.take();
                    assign.guard = match cur_guard {
                        None => Some(group_go.clone()),
                        Some(g) => Some(g & group_go.clone()),
                    };
                }
            }
        }

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
