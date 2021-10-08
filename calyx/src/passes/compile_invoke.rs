use crate::errors::Error;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Attributes, LibrarySignatures};
use crate::structure;

/// Compiles [`ir::Invoke`](crate::ir::Invoke) statements into an [`ir::Enable`](crate::ir::Enable)
/// that runs the invoked component.
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
        let mut builder = ir::Builder::new(comp, ctx);

        let invoke_group = builder.add_group("invoke");

        // comp.go = 1'd1;
        // invoke[done] = comp.done;
        structure!(builder;
            let one = constant(1, 1);
        );

        let cell = s.comp.borrow();
        let go_port = cell
            .find_with_attr("go")
            .ok_or_else(|| Error::MalformedControl(format!("Invoked component `{}` does not have a port with attribute @go", cell.name())))?;
        let done_port = cell.find_with_attr("done")
            .ok_or_else(|| Error::MalformedControl(format!("Invoked component `{}` does not have a port with attribute @done", cell.name())))?;
        let go_assign = builder.build_assignment(
            go_port,
            one.borrow().get("out"),
            ir::Guard::True,
        );
        let done_assign = builder.build_assignment(
            invoke_group.borrow().get("done"),
            done_port,
            ir::Guard::True,
        );
        let mut enable_assignments = vec![go_assign, done_assign];

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
            .chain(enable_assignments.drain(..))
            .collect();
        invoke_group.borrow_mut().assignments = assigns;

        // Copy "static" annotation from the `invoke` statement if present
        if let Some(time) = s.attributes.get("static") {
            invoke_group.borrow_mut().attributes.insert("static", *time);
        }

        let mut en = ir::Enable {
            group: invoke_group,
            attributes: Attributes::default(),
        };
        if let Some(time) = s.attributes.get("static") {
            en.attributes.insert("static", *time);
        }

        Ok(Action::Change(ir::Control::Enable(en)))
    }
}
