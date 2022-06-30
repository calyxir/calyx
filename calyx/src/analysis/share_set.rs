use crate::ir;
use crate::ir::RRC;
use std::collections::HashSet;

/// The type names of all components and primitives marked with "state_share".
/// Methods can be used to determine whether a cell is actually shareable or not
/// Used by both `live_range_analysis.rs` and `infer_share.rs`
#[derive(Default)]
pub struct ShareSet {
    shareable: HashSet<ir::Id>,
}

impl ShareSet {
    pub fn new(set: HashSet<ir::Id>) -> Self {
        ShareSet { shareable: set }
    }

    pub fn from_context(ctx: &ir::Context) -> Self {
        let mut state_shareable = HashSet::new();
        for prim in ctx.lib.signatures() {
            if prim.attributes.has("state_share") {
                state_shareable.insert(prim.name.clone());
            }
        }
        // add state_share=1 user defined components to the state_shareable set
        for comp in &ctx.components {
            if comp.attributes.has("state_share") {
                state_shareable.insert(comp.name.clone());
            }
        }
        ShareSet {
            shareable: state_shareable,
        }
    }

    pub fn add(&mut self, id: ir::Id) {
        self.shareable.insert(id);
    }

    //given a set of shareable and a cell, determines whether cell's
    //type is shareable or not
    pub fn is_shareable_component(&self, cell: &RRC<ir::Cell>) -> bool {
        if let Some(type_name) = cell.borrow().type_name() {
            self.shareable.contains(type_name)
        } else {
            false
        }
    }
}
