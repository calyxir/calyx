use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, LibrarySignatures};

use crate::analysis::ReachingDefinitionAnalysis;

#[derive(Default)]
pub struct RegisterUnsharing {}

impl Named for RegisterUnsharing {
    fn name() -> &'static str {
        "register-unsharing"
    }

    fn description() -> &'static str {
        "Split apart shared values into separate regsters"
    }
}

impl Visitor for RegisterUnsharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &LibrarySignatures,
    ) -> VisResult {
        let analysis =
            ReachingDefinitionAnalysis::new(&comp, &comp.control.borrow());

        for (group, z) in &analysis.reach {
            println!("Group {}", group);
            println!("  {:?}", z);
        }

        for (x, y) in analysis.calculate_overlap() {
            println!("Overlapping defns for {}", x);
            for def in y {
                println!("   {:?}\n", def);
            }
        }
        Ok(Action::Stop)
    }
}
