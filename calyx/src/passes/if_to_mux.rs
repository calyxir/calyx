use crate::ast::{If, Namespace};
use crate::passes::visitor::{Visitable, Visitor};

pub struct Muxify {}

impl Visitor<()> for Muxify {
    fn new() -> Muxify {
        Muxify {}
    }

    fn name(&self) -> String {
        "Muxify".to_string()
    }

    fn start_if(&mut self, con_if: &mut If) -> Result<(), ()> {
        Ok(())
    }
}

pub fn if_to_mux(n: &mut Namespace) -> Muxify {
    let mut mux = Muxify {};
    n.visit(&mut mux)
        .unwrap_or_else(|x| panic!("Muxify pass failed: {:?}", x));
    mux
}
