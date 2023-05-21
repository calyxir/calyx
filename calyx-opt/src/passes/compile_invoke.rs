use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir::structure;
use calyx_ir::{self as ir, Attributes, LibrarySignatures};
use calyx_utils::{CalyxResult, Error};
use itertools::Itertools;
use std::rc::Rc;

// given `cell_ref` returns the `go` port of the cell (if it only has one `go` port),
// or an error otherwise
fn get_go_port(cell_ref: ir::RRC<ir::Cell>) -> CalyxResult<ir::RRC<ir::Port>> {
    let cell = cell_ref.borrow();

    let name = cell.name();

    // Get the go port
    let mut go_ports = cell.find_all_with_attr(ir::NumAttr::Go).collect_vec();
    if go_ports.len() > 1 {
        return Err(Error::malformed_control(format!("Invoked component `{name}` defines multiple @go signals. Cannot compile the invoke")));
    } else if go_ports.is_empty() {
        return Err(Error::malformed_control(format!("Invoked component `{name}` does not define a @go signal. Cannot compile the invoke")));
    }

    Ok(go_ports.pop().unwrap())
}

// given inputs and outputs (of the invoke), and the `enable_assignments` (e.g., invoked_component.go = 1'd1)
// and a cell, builds the assignments for the corresponding group
fn build_assignments<T>(
    inputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
    outputs: &mut Vec<(ir::Id, ir::RRC<ir::Port>)>,
    mut enable_assignments: Vec<ir::Assignment<T>>,
    builder: &mut ir::Builder,
    cell: &ir::Cell,
) -> Vec<ir::Assignment<T>> {
    inputs
        .drain(..)
        .map(|(inp, p)| {
            builder.build_assignment(cell.get(inp), p, ir::Guard::True)
        })
        .chain(outputs.drain(..).map(|(out, p)| {
            builder.build_assignment(p, cell.get(out), ir::Guard::True)
        }))
        .chain(enable_assignments.drain(..))
        .collect()
}

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
        let go_port = get_go_port(Rc::clone(&s.comp))?;

        // Get the done ports
        let mut done_ports =
            cell.find_all_with_attr(ir::NumAttr::Done).collect_vec();
        if done_ports.len() > 1 {
            return Err(Error::malformed_control(format!("Invoked component `{name}` defines multiple @done signals. Cannot compile the invoke")));
        } else if done_ports.is_empty() {
            return Err(Error::malformed_control(format!("Invoked component `{name}` does not define a @done signal. Cannot compile the invoke")));
        }

        // Build assignemnts
        let go_assign = builder.build_assignment(
            go_port,
            one.borrow().get("out"),
            ir::Guard::True,
        );
        let done_assign = builder.build_assignment(
            invoke_group.borrow().get("done"),
            done_ports.pop().unwrap(),
            ir::Guard::True,
        );
        let enable_assignments = vec![go_assign, done_assign];

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = build_assignments(
            &mut s.inputs,
            &mut s.outputs,
            enable_assignments,
            &mut builder,
            cell,
        );
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
        if let Some(time) = s.attributes.get(ir::NumAttr::Static) {
            invoke_group
                .borrow_mut()
                .attributes
                .insert(ir::NumAttr::Static, time);
        }

        let mut en = ir::Enable {
            group: invoke_group,
            attributes: Attributes::default(),
        };
        if let Some(time) = s.attributes.get(ir::NumAttr::Static) {
            en.attributes.insert(ir::NumAttr::Static, time);
        }

        Ok(Action::change(ir::Control::Enable(en)))
    }

    fn static_invoke(
        &mut self,
        s: &mut ir::StaticInvoke,
        comp: &mut ir::Component,
        ctx: &LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        let mut builder = ir::Builder::new(comp, ctx);

        let invoke_group = builder.add_static_group("static_invoke", s.latency);

        if !s.ref_cells.is_empty() {
            return Err(Error::malformed_structure(format!(
                "Invoke statement contains ref cell. Run {} before this pass",
                super::CompileRef::name()
            )));
        }

        // comp.go = 1'd1;
        structure!(builder;
            let one = constant(1, 1);
        );

        // Get the go port
        let go_port = get_go_port(Rc::clone(&s.comp))?;

        // Build assignemnts
        let go_assign = builder.build_assignment(
            go_port,
            one.borrow().get("out"),
            ir::Guard::True,
        );

        let enable_assignments = vec![go_assign];

        // Generate argument assignments
        let cell = &*s.comp.borrow();
        let assigns = build_assignments(
            &mut s.inputs,
            &mut s.outputs,
            enable_assignments,
            &mut builder,
            cell,
        );
        invoke_group.borrow_mut().assignments = assigns;

        let en = ir::StaticEnable {
            group: invoke_group,
            attributes: Attributes::default(),
        };

        Ok(Action::StaticChange(Box::new(ir::StaticControl::Enable(
            en,
        ))))
    }
}
