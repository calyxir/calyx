use crate::frontend::library::ast::LibrarySignatures;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir;
use crate::{build_assignments, structure, guard};
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileInvoke;

impl Named for CompileInvoke {
    fn name() -> &'static str {
        "compile-invoke"
    }

    fn description() -> &'static str {
        "Rewrites invoke statements to group enables"
    }
}

impl Visitor for CompileInvoke {
    fn invoke(
        &mut self,
        s: &mut ir::Invoke,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
    ) -> VisResult {
        let mut builder = ir::Builder::from(comp, ctx, false);

        let invoke_group = builder.add_group("invoke", HashMap::new());

        // Generate state elements to make sure that component is only run once.
        // comp.go = once.out != 1 ? 1;
        // once.in = once.out != 1 ? comp.done;
        // once.write_en = once.out != 1 ? comp.done;
        // invoke[done] = once.out == 1
        structure!(builder;
            let once = prim std_reg(1);
            let done_const = constant(1, 1);
            let zero = constant(0, 1);
        );

        let cell = &s.comp;
        let is_done = guard!(once["out"]).eq(guard!(done_const["out"]));
        let is_not_done = !is_done.clone();
        let mut once_assignments = build_assignments!(builder;
            cell["go"] = is_not_done ? done_const["out"];
            once["in"] = is_not_done ? cell["done"];
            once["write_en"] = is_not_done ? cell["done"];
            invoke_group["done"] = is_done ? done_const["out"];
        );

        // CLEANUP: Set once to 0;
        // once.in = once.out == 1 ? 0;
        // once.write_en = once.out == 1 ? 1;
        let mut cleanup = build_assignments!(builder;
            once["in"] = is_done ? zero["out"];
            once["write_en"] = is_done ? done_const["out"];
        );

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = s
            .inputs
            .drain(..)
            .into_iter()
            .map(|(inp, p)| {
                builder.build_assignment(cell.get(inp), p, ir::Guard::True)
            })
            .chain(s.outputs.drain(..).into_iter().map(|(out, p)| {
                builder.build_assignment(p, cell.get(out), ir::Guard::True)
            }))
            .chain(once_assignments.drain(..))
            .chain(cleanup.drain(..))
            .collect();
        invoke_group.borrow_mut().assignments = assigns;

        Ok(Action::Change(ir::Control::enable(invoke_group)))
    }
}
