use super::visitor::{Changes, Visitor};
use crate::lang::ast::{Component, Portdef};

pub struct LatencyInsenstive {}

impl LatencyInsenstive {
    pub fn new() -> Self {
        LatencyInsenstive {}
    }
}

impl Visitor<()> for LatencyInsenstive {
    fn name(&self) -> String {
        "Latency Insenstive".to_string()
    }

    fn start(
        &mut self,
        _comp: &mut Component,
        changes: &mut Changes,
    ) -> Result<(), ()> {
        let val = Portdef {
            name: "valid".to_string(),
            width: 1,
        };
        let rdy = Portdef {
            name: "ready".to_string(),
            width: 1,
        };

        changes.add_input_port(val);
        changes.add_output_port(rdy);

        changes.commit();
        Err(())
    }
}
