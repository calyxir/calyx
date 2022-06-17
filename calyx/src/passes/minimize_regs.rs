use super::sharing_components::ShareComponents;
use crate::errors::CalyxResult;
use crate::{
    analysis::{LiveRangeAnalysis, ReadWriteSet},
    ir::{self, traversal::ConstructVisitor, traversal::Named, CloneName},
};
use std::collections::{HashMap, HashSet};

/// Given a [LiveRangeAnalysis] that specifies the registers alive at each
/// group, minimize the registers used for each component.
///
/// This works by constructing an interference graph for each alive register.
/// If two registers are ever alive at the same time, then there is an edge
/// between them in the interference graph. Additionally, if two registers
/// are different sizes, then there is an edge between them.
///
/// A greedy graph coloring algorithm on the interference graph
/// is used to assign each register a name.
///
/// This pass only renames uses of registers. [crate::passes::DeadCellRemoval] should be run after this
/// to actually remove the register definitions.
pub struct MinimizeRegs {
    live: LiveRangeAnalysis,
    rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    /// Set of state shareable components (as type names)
    state_shareable: HashSet<ir::Id>,

    /// Mapping from the name of a group to the cells that it uses.
    used_cells_map: HashMap<ir::Id, Vec<ir::Id>>,

    /// Set of shareable components.
    shareable: HashSet<ir::Id>,
}

impl Named for MinimizeRegs {
    fn name() -> &'static str {
        "minimize-regs"
    }
    fn description() -> &'static str {
        "use the fewest possible registers"
    }
}

impl ConstructVisitor for MinimizeRegs {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut state_shareable = HashSet::new();
        let mut shareable = HashSet::new();
        // add state_share=1 primitives to the state_shareable set
        for prim in ctx.lib.signatures() {
            if let Some(&1) = prim.attributes.get("share") {
                shareable.insert(prim.name.clone());
            } else if let Some(&1) = prim.attributes.get("state_share") {
                state_shareable.insert(prim.name.clone());
            }
        }

        // add share=1 user defined components to the shareable_components set
        for comp in &ctx.components {
            if let Some(&1) = comp.attributes.get("share") {
                shareable.insert(comp.name.clone());
            }
        }

        Ok(MinimizeRegs {
            live: LiveRangeAnalysis::default(),
            rewrites: HashMap::new(),
            state_shareable,
            used_cells_map: HashMap::new(),
            shareable,
        })
    }

    fn clear_data(&mut self) {
        self.rewrites = HashMap::new();
        self.live = LiveRangeAnalysis::default();
        self.used_cells_map = HashMap::new();
    }
}

//Given cell, cont_cell, and shareable_components, returns true if
//cell is shareable (as determined by shareable_components) and is not
//a continuous cell
fn share_filter(cell: &ir::Cell, shareable: &HashSet<ir::Id>) -> bool {
    if let Some(type_name) = cell.type_name() {
        shareable.contains(type_name)
    } else {
        false
    }
}

impl ShareComponents for MinimizeRegs {
    fn initialize(
        &mut self,
        comp: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        self.live = LiveRangeAnalysis::new(
            comp,
            &*comp.control.borrow(),
            self.state_shareable.clone(),
            self.shareable.clone(),
        );

        //Following code is from resource_sharing pass
        let group_uses = comp.groups.iter().map(|group| {
            (
                group.clone_name(),
                ReadWriteSet::uses(group.borrow().assignments.iter())
                    .filter(|cell| {
                        share_filter(&cell.borrow(), &self.shareable)
                    })
                    .map(|cell| cell.clone_name())
                    .collect::<Vec<_>>(),
            )
        });
        let cg_uses = comp.comb_groups.iter().map(|cg| {
            (
                cg.clone_name(),
                ReadWriteSet::uses(cg.borrow().assignments.iter())
                    .filter(|cell| {
                        share_filter(&cell.borrow(), &self.shareable)
                    })
                    .map(|cell| cell.clone_name())
                    .collect::<Vec<_>>(),
            )
        });
        self.used_cells_map = group_uses.chain(cg_uses).collect();
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        let mut state_shareable: Vec<ir::Id> =
            self.live.get(group_name).iter().cloned().collect();

        //resource-sharing code
        let shareable = self
            .used_cells_map
            .get(group_name)
            .unwrap_or_else(|| {
                panic!("Missing used cells for group: {}", group_name)
            })
            .clone();

        state_shareable.extend(shareable);

        state_shareable
    }

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
        if let Some(name) = cell.type_name() {
            self.state_shareable.contains(name) || self.shareable.contains(name)
        } else {
            false
        }
    }

    fn custom_conflicts<F>(&self, comp: &ir::Component, mut add_conflicts: F)
    where
        F: FnMut(Vec<ir::Id>),
    {
        for group in comp.groups.iter() {
            let conflicts = self.live.get(group.borrow().name());
            add_conflicts(conflicts.iter().cloned().collect());
        }

        //resource-sharing code
        for used in self.used_cells_map.values() {
            add_conflicts(used.clone())
        }
    }

    fn set_rewrites(&mut self, rewrites: HashMap<ir::Id, ir::RRC<ir::Cell>>) {
        self.rewrites = rewrites;
    }

    fn get_rewrites(&self) -> &HashMap<ir::Id, ir::RRC<ir::Cell>> {
        &self.rewrites
    }
}
