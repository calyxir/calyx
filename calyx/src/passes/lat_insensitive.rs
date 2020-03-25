use crate::lang::{component::Component, context::Context};
use crate::passes::visitor::{Action, VisResult, Visitor};

#[derive(Default)]
pub struct LatencyInsenstive {}

impl Visitor for LatencyInsenstive {
    fn name(&self) -> String {
        "Latency Insenstive".to_string()
    }

    fn start(&mut self, comp: &mut Component, _c: &Context) -> VisResult {
        comp.add_input(("valid", 1));
        comp.add_input(("clk", 1));
        comp.add_output(("ready", 1));

        Ok(Action::Stop)
    }
}
