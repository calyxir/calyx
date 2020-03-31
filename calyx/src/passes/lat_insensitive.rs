use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, Named, VisResult, Visitor};

pub struct LatencyInsenstive {}

impl Default for LatencyInsenstive {
    fn default() -> Self {
        LatencyInsenstive {}
    }
}

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
        comp.add_input(("valid", 1));
        comp.add_output(("ready", 1));

        Ok(Action::Stop)
    }
}
