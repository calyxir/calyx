use super::visitor::{Changes, Visitor};
use crate::lang::ast::Portdef;

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

    fn start(&mut self, changes: &mut Changes) -> Result<(), ()> {
        let val = Portdef {
            name: "valid".to_string(),
            width: 1,
        };
        let reset = Portdef {
            name: "reset".to_string(),
            width: 1,
        };
        let clk = Portdef {
            name: "clk".to_string(),
            width: 1,
        };
        let rdy = Portdef {
            name: "ready".to_string(),
            width: 1,
        };

        changes.add_input_port(val);
        changes.add_input_port(reset);
        changes.add_input_port(clk);
        changes.add_output_port(rdy);

        // return err to avoid touching every control node
        Err(())
    }
}
