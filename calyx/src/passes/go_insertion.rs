use crate::guard;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, LibrarySignatures};

#[derive(Default)]
/// Add the group's `go` signal into the guards of all non-hole assignments
/// of this group.
///
/// For example, the pass transforms this FuTIL program:
/// ```
/// group foo {
///     x.in = cond ? 32'd1;
///     foo[done] = reg.done;
/// }
/// ```
/// into:
/// ```
/// group foo {
///     x.in = cond & foo[go] ? 32'd1;
///     foo[done] = reg.done;
/// }
/// ```
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
        _c: &LibrarySignatures,
    ) -> VisResult {
        for group in &comp.groups {
            let group_go = guard!(group["go"]);
            // Detach the group's assignments so we can drop the mutable access to it.
            let mut group_assigns =
                group.borrow_mut().assignments.drain(..).collect::<Vec<_>>();
            for assign in group_assigns.iter_mut() {
                let dst = assign.dst.borrow();
                if !(dst.is_hole() && dst.name == "done") {
                    assign.guard &= group_go.clone();
                }
            }
            group.borrow_mut().assignments = group_assigns;
        }

        // This pass doesn't modify any control.
        Ok(Action::Stop)
    }
}
