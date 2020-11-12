use super::LiveRangeAnalysis;
use crate::frontend::library::ast as lib;
use crate::ir::{
    self,
    traversal::{Named, Visitor},
};
use ir::{
    traversal::{Action, VisResult},
    Component,
};

#[derive(Default)]

/// Minimize use of registers
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
}

impl Named for MinimizeRegs {
    fn name() -> &'static str {
        "minimize-regs"
    }
    fn description() -> &'static str {
        "use the fewest possible registers"
    }
}

impl Visitor<()> for MinimizeRegs {
    fn start(
        &mut self,
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        Ok(Action::continue_default())
    }
}
