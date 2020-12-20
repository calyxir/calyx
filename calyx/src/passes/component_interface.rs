use crate::errors::Error;
use crate::frontend::library::ast as lib;
use crate::ir;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::{build_assignments, guard, structure};
use std::rc::Rc;

#[derive(Default)]
/// Wires up the `go` and `done` holes for FuTIL programs with a single
/// enable to the component `go` and `done` ports.
///
/// For example:
/// ```
/// component main(go: 1) -> (done: 1) {
///     cells { .. }
///     wires {
///         group only_group { .. }
///     }
///     control { only_group; }
/// }
/// ```
/// is transformed into:
/// ```
/// component main(go: 1) -> (done: 1) {
///     cells { .. }
///     wires {
///         group only_group { .. }
///         only_group[go] = go;
///         done = only_group[done];
///     }
///     control { only_group; }
/// }
/// ```
pub struct ComponentInterface;

impl Named for ComponentInterface {
    fn name() -> &'static str {
        "component-interface-inserter"
    }

    fn description() -> &'static str {
        "wire up a single enable to the go/done interface in a component"
    }
}

impl Visitor for ComponentInterface {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        ctx: &lib::LibrarySignatures,
    ) -> VisResult {
        let control_ref = Rc::clone(&comp.control);
        let control = control_ref.borrow();

        if let ir::Control::Enable(data) = &*control {
            let this = Rc::clone(&comp.signature);
            let mut builder = ir::Builder::from(comp, ctx, false);
            let group = &data.group;

            structure!(builder;
                let one = constant(1, 1);
            );
            let group_done = guard!(group["done"]);
            let mut assigns = build_assignments!(builder;
                group["go"] = ? this["go"];
                this["done"] = group_done ? one["out"];
            );
            comp.continuous_assignments.append(&mut assigns);

            Ok(Action::Stop)
        } else if let ir::Control::Empty(..) = &*control {
            Ok(Action::Stop)
        } else {
            Err(Error::MalformedControl(
                "ComponentInterface: Structure has more than one group"
                    .to_string(),
            ))
        }
    }
}
