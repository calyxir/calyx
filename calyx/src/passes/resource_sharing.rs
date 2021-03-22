use super::sharing_components::ShareComponents;
use crate::analysis;
use crate::ir::{self, traversal::Named, RRC};
use std::collections::HashMap;

#[derive(Default)]
/// Rewrites groups to share cells marked with the "share" attribute
/// when the groups are guaranteed to never run in parallel.
pub struct ResourceSharing {
    /// Mapping from the name of a group to the cells that it uses.
    shareable_components: HashMap<ir::Id, Vec<ir::Id>>,

    /// This is used to rewrite all uses of `old_cell` with `new_cell` in the group.
    rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>,
}

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "shares resources between groups that don't execute in parallel"
    }
}

impl ShareComponents for ResourceSharing {
    fn initialize(
        &mut self,
        component: &ir::Component,
        sigs: &ir::LibrarySignatures,
    ) {
        self.shareable_components = component
            .groups
            .iter()
            .map(|group| {
                (
                    group.borrow().name.clone(),
                    analysis::ReadWriteSet::uses(&group.borrow().assignments)
                        .into_iter()
                        .filter(|cell| self.cell_filter(&cell.borrow(), sigs))
                        .map(|cell| cell.borrow().name.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.shareable_components[group_name].clone()
    }

    fn cell_filter(
        &self,
        cell: &ir::Cell,
        sigs: &ir::LibrarySignatures,
    ) -> bool {
        if let ir::CellType::Primitive {
            name: prim_type, ..
        } = &cell.prototype
        {
            sigs.get_primitive(&prim_type).attributes.get("share") == Some(&1)
        } else {
            false
        }
    }

    fn custom_conflicts(
        &self,
        _comp: &ir::Component,
        graph: &mut analysis::GraphColoring<ir::Id>,
    ) {
        for confs in self.shareable_components.values() {
            graph.insert_conflicts(confs.iter());
        }
    }

    fn set_rewrites(&mut self, rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>) {
        self.rewrites = rewrites;
    }

    fn get_rewrites(&self) -> &[(RRC<ir::Cell>, RRC<ir::Cell>)] {
        &self.rewrites
    }
}
