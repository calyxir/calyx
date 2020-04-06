use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

#[derive(Default)]
pub struct LatencyInsenstive {}

impl Named for LatencyInsenstive {
    fn name() -> &'static str {
        "latency-insenstive"
    }

    fn description() -> &'static str {
        "Added a latency insenstive interface to all top level components"
    }
}

impl Visitor for LatencyInsenstive {
    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        if !comp.signature.has_input("valid") {
            comp.add_input(("valid", 1));
        }

        if !comp.signature.has_input("clk") {
            comp.add_input(("clk", 1));
        }

        if !comp.signature.has_output("ready") {
            comp.add_output(("ready", 1));
        }

        Ok(Action::Stop)
    }
}
