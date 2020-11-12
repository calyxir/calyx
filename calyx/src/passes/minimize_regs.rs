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

/// Minimize use of registers
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
}

impl MinimizeRegs {
    pub fn new(live: LiveRangeAnalysis) -> Self {
        MinimizeRegs { live }
    }
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
    fn start_enable(
        &mut self,
        enable: &mut ir::Enable,
        _data: (),
        _comp: &mut Component,
        _sigs: &lib::LibrarySignatures,
    ) -> VisResult<()> {
        let name = &enable.group.borrow().name;
        eprintln!("{}", name.to_string());
        eprintln!(
            "  {}",
            self.live
                .get(&enable.group.borrow())
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(Action::continue_default())
    }
}
