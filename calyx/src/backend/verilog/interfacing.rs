use crate::errors::Error;
use crate::lang::ast::Control;
use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct Interfacing;

impl Named for Interfacing {
    fn name() -> &'static str {
        "Interfacing"
    }

    fn description() -> &'static str {
        "Add ready, valid, clk signals to main module"
    }
}

impl Visitor for Interfacing {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        let group_idx = match &comp.control {
            Control::Enable { data } => comp.structure.get_idx(&data.group)?,
            _ => {
                return Err(Error::MalformedControl(
                    "Interfacing expects a single top-level enable".to_string(),
                ))
            }
        };

        comp.add_input(("valid", 1))?;
        comp.add_input(("clk", 1))?;
        comp.add_output(("ready", 1))?;

        let this = comp.structure.get_this();
        comp.structure
            .insert_edge(this, "valid", group_idx, "valid")?;
        comp.structure.insert_edge(this, "clk", group_idx, "clk")?;
        comp.structure
            .insert_edge(group_idx, "ready", this, "ready")?;

        Ok(Action::Change(Control::empty()))
    }
}
