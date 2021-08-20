use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Attributes, LibrarySignatures};
use crate::{build_assignments, structure};

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

        let cell = &s.comp;
        let mut enable_assignments = build_assignments!(builder;
            cell["go"] = ? one["out"];
            invoke_group["done"] = ? cell["done"];
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
