use crate::lang::ast::If;
use crate::passes::visitor::{Changes, Visitor};

pub struct Muxify {}

impl Muxify {
    pub fn new() -> Muxify {
        Muxify {}
    }
}

impl Visitor<()> for Muxify {
    fn name(&self) -> String {
        "Muxify".to_string()
    }

    fn start_if(
        &mut self,
        _con_if: &mut If,
        _changes: &mut Changes,
    ) -> Result<(), ()> {
        Ok(())
    }
}
