use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, VisResult, Visitor};

pub struct LatencyInsenstive {}

impl Default for LatencyInsenstive {
    fn default() -> Self {
        LatencyInsenstive {}
    }
}

impl Visitor for LatencyInsenstive {
    fn name(&self) -> String {
        "Latency Insenstive".to_string()
    }

    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        comp.add_input(("valid", 1));
        comp.add_output(("ready", 1));

        Ok(Action::Stop)
    }
}
