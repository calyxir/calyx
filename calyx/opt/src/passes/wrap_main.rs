use std::borrow::BorrowMut;

use crate::traversal::{Action, Named, VisResult, Visitor};
use calyx_ir as ir;
use ir::build_assignments;

#[derive(Default)]
/// If the top-level component is not named `main`, adds a new `main` component
/// and makes it the top-level component.
/// This is useful because a lot of our tools rely on the name `main` being the design under test (DUT).
///
/// For more information, see https://github.com/calyxir/calyx/issues/1376
pub struct WrapMain;

impl Named for WrapMain {
    fn name() -> &'static str {
        "wrap-main"
    }

    fn description() -> &'static str {
        "If the top-level component is not named `main`, adds a new `main` component and makes it the top-level component."
    }
}

impl Visitor for WrapMain {
    fn precondition(ctx: &calyx_ir::Context) -> Option<String>
    where
        Self: Sized,
    {
        if ctx.entrypoint().name == "main" {
            Some("Top-level component is already named `main'".to_string())
        } else {
            None
        }
    }

    fn start_context(&mut self, ctx: &mut ir::Context) -> VisResult {
        let entry = ctx.entrypoint();
        let sig = entry.signature.borrow();

        // If any of the ports in the entrypoint component are non-interface ports, refuse to run this pass.
        if let Some(p) = sig.ports.iter().find(|p| {
            let port = p.borrow();
            let attr = &port.attributes;
            !(attr.has(ir::BoolAttr::Clk)
                || attr.has(ir::BoolAttr::Reset)
                || attr.has(ir::NumAttr::Go)
                || attr.has(ir::NumAttr::Done))
        }) {
            let pn = p.borrow().name;
            log::warn!(
                "Entrypoint component `{}' has non-interface port `{}'. Cannot wrap it in `main' component. The component might not simulate with the Calyx test bench or generate results with the synthesis scripts without modification.",
                entry.name,
                pn
            );
            return Ok(Action::Stop);
        }
        let entry_name = entry.name;
        let mut ports = sig.get_signature();
        ports
            .iter_mut()
            .for_each(|pd| pd.direction = pd.direction.reverse());
        drop(sig);

        // Remove top-level attribute from previous component
        ctx.entrypoint_mut()
            .attributes
            .remove(ir::BoolAttr::TopLevel);

        // Create a new `main' component
        let mut main = ir::Component::new("main", vec![], true, false, None);
        main.borrow_mut()
            .attributes
            .insert(ir::BoolAttr::TopLevel, 1);

        // Add the original top-level component as a cell to the main component.
        {
            let mut builder = ir::Builder::new(&mut main, &ctx.lib);
            let comp = builder.add_component(entry_name, entry_name, ports);
            let main_sig = builder.component.signature.clone();
            let cont_assigns = build_assignments!(builder;
                comp["go"] = ? main_sig["go"];
                main_sig["done"] = ? comp["done"];
            );
            builder
                .component
                .continuous_assignments
                .extend(cont_assigns);
        }

        // Update the context entrypoint to be the main component
        ctx.entrypoint = main.name;
        ctx.components.push(main);

        // Purely context directed pass
        Ok(Action::Stop)
    }
}
