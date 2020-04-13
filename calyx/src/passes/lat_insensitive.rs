use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct LatencyInsensitive {}

impl Named for LatencyInsensitive {
    fn name() -> &'static str {
        "latency-insensitive"
    }

    fn description() -> &'static str {
        "Added a latency insenstive interface to all top level components"
    }
}

impl Visitor for LatencyInsensitive {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        comp.add_input(("valid", 1))?;
        comp.add_input(("clk", 1))?;
        comp.add_output(("ready", 1))?;

        Ok(Action::Stop)
    }
}
