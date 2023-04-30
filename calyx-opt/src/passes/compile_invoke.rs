use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::structure;
use calyx_ir::{self as ir, Attributes, LibrarySignatures};
use calyx_utils::Error;
use itertools::Itertools;

/// Compiles [`ir::Invoke`](calyx_ir::Invoke) statements into an [`ir::Enable`](calyx_ir::Enable)
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
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);

        let invoke_group = builder.add_group("invoke");

        if !s.ref_cells.is_empty() {
            return Err(Error::malformed_structure(format!(
                "Invoke statement contains ref cell. Run {} before this pass",
                super::CompileRef::name()
            )));
        }

        // comp.go = 1'd1;
        // invoke[done] = comp.done;
        structure!(builder;
            let one = constant(1, 1);
        );

        let cell = s.comp.borrow();
        let name = cell.name();

        // Get the go port
        let mut go_ports = cell.find_all_with_attr("go").collect_vec();
        if go_ports.len() > 1 {
            return Err(Error::malformed_control(format!("Invoked component `{name}` defines multiple @go signals. Cannot compile the invoke")));
        } else if go_ports.is_empty() {
            return Err(Error::malformed_control(format!("Invoked component `{name}` does not define a @go signal. Cannot compile the invoke")));
        }

        // Get the done ports
        let mut done_ports = cell.find_all_with_attr("done").collect_vec();
        if done_ports.len() > 1 {
            return Err(Error::malformed_control(format!("Invoked component `{name}` defines multiple @done signals. Cannot compile the invoke")));
        } else if done_ports.is_empty() {
            return Err(Error::malformed_control(format!("Invoked component `{name}` does not define a @done signal. Cannot compile the invoke")));
        }

        // Build assignemnts
        let go_assign = builder.build_assignment(
            go_ports.pop().unwrap(),
            one.borrow().get("out"),
            ir::Guard::True,
        );
        let done_assign = builder.build_assignment(
            invoke_group.borrow().get(ir::Attribute::Done),
            done_ports.pop().unwrap(),
            ir::Guard::True,
        );
        let mut enable_assignments = vec![go_assign, done_assign];

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = s
            .inputs
            .drain(..)
            .map(|(inp, p)| {
                builder.build_assignment(cell.get(inp), p, ir::Guard::True)
            })
            .chain(s.outputs.drain(..).map(|(out, p)| {
                builder.build_assignment(p, cell.get(out), ir::Guard::True)
            }))
            .chain(enable_assignments.drain(..))
            .collect();
        invoke_group.borrow_mut().assignments = assigns;

        // Add assignments from the attached combinational group
        if let Some(cgr) = &s.comb_group {
            let cg = &*cgr.borrow();
            invoke_group
                .borrow_mut()
                .assignments
                .extend(cg.assignments.iter().cloned())
        }

        // Copy "static" annotation from the `invoke` statement if present
        if let Some(time) = s.attributes.get(ir::Attribute::Static) {
            invoke_group.borrow_mut().attributes.insert("static", *time);
        }

        let mut en = ir::Enable {
            group: invoke_group,
            attributes: Attributes::default(),
        };
        if let Some(time) = s.attributes.get(ir::Attribute::Static) {
            en.attributes.insert("static", *time);
        }

        Ok(Action::change(ir::Control::Enable(en)))
    }
}
