use crate::analysis::ReachingDefinitionAnalysis;
use crate::ir::traversal::{Action, Named, VisResult, Visitor};
use crate::ir::{self, Builder, LibrarySignatures};
use crate::utils::NameGenerator;
use std::collections::{HashMap, HashSet};

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

struct BookKeeper {
    name_gen: NameGenerator,
    analysis: ReachingDefinitionAnalysis,
    widths: HashMap<ir::Id, u64>,
}

impl BookKeeper {
    fn gen_name(&mut self, id: &ir::Id) -> ir::Id {
        ir::Id::new(self.name_gen.gen_name(format!("unshr_{}", id)), None)
    }

    fn new(comp: &ir::Component) -> Self {
        let widths = comp
            .cells
            .iter()
            .filter_map(|c| {
                if let ir::CellType::Primitive { name, .. } =
                    &c.borrow().prototype
                {
                    if name == "std_reg" {
                        if let Some(in_port) = c.borrow().find("in") {
                            Some((
                                c.borrow().name.clone(),
                                in_port.borrow().width,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        // (griffin) I'm sorry for the above.
        // There's probably a cleaner way to write this
        // TODO(griffin): fix?

        let analysis =
            ReachingDefinitionAnalysis::new(&comp, &comp.control.borrow());
        let name_gen = NameGenerator::default();

        Self {
            name_gen,
            analysis,
            widths,
        }
    }

    fn create_new_regs(&mut self, builder: &mut Builder) {
        let overlap = self.analysis.calculate_overlap();

        for (name, sets) in &overlap {
            if sets.len() > 1 {
                for defs in &sets[1..] {
                    builder.add_primitive(
                        self.gen_name(name),
                        "std_reg",
                        &[*self.widths.get(name).unwrap()],
                    );
                }
            }
        }
    }
}

impl Visitor for RegisterUnsharing {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _c: &LibrarySignatures,
    ) -> VisResult {
        let mut bookkeeper = BookKeeper::new(comp);
        let mut builder = Builder::from(comp, _c, false);

        bookkeeper.create_new_regs(&mut builder);
        for (group, z) in &bookkeeper.analysis.reach {
            println!("Group {}", group);
            println!("  {:?}", z);
        }

        for (x, y) in &bookkeeper.analysis.calculate_overlap() {
            println!("Overlapping defns for {}", x);
            for def in y {
                println!("   {:?}\n", def);
            }
        }
        Ok(Action::Stop)
    }
}
