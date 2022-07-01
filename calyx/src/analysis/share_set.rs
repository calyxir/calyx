use crate::ir;
use crate::ir::RRC;
use std::collections::HashSet;

/// Stores a Hashset that contains the type names of all components and primitives
/// marked with either "share" or "state_share",depending on what the user wants.
/// Methods implemented by this struct can
/// be used to determine whether a given cell is shareable or not
/// Used by `live_range_analysis.rs`, `cell_share.rs`, and `infer_share.rs`
#[derive(Default, Clone)]
pub struct ShareSet {
    shareable: HashSet<ir::Id>,
    is_state_share: bool,
}

impl ShareSet {
    pub fn new(shareable: HashSet<ir::Id>, is_state_share: bool) -> Self {
        ShareSet {
            shareable,
            is_state_share,
        }
    }

    ///Constructs a shareset from the context. Looks for "state_share" types if
    ///is_state_share is true, and "share" types otherwise.
    pub fn from_context(ctx: &ir::Context, is_state_share: bool) -> Self {
        let keyword = if is_state_share {
            "state_share"
        } else {
            "share"
        };
        let mut shareable = HashSet::new();
        for prim in ctx.lib.signatures() {
            if prim.attributes.has(keyword) {
                shareable.insert(prim.name.clone());
            }
        }
        // add state_share=1 user defined components to the state_shareable set
        for comp in &ctx.components {
            if comp.attributes.has(keyword) {
                shareable.insert(comp.name.clone());
            }
        }
        ShareSet {
            shareable,
            is_state_share,
        }
    }

    ///Adds id to self
    pub fn add(&mut self, id: ir::Id) {
        self.shareable.insert(id);
    }

    ///Checks if id contains self
    pub fn contains(&self, id: &ir::Id) -> bool {
        self.shareable.contains(id)
    }

    ///Returns whether or not this instance is state_share
    pub fn is_state_share(&self) -> bool {
        self.is_state_share
    }

    ///Given a set of shareable and a cell, determines whether cell's
    ///type is shareable or not
    pub fn is_shareable_component(&self, cell: &RRC<ir::Cell>) -> bool {
        if let Some(type_name) = cell.borrow().type_name() {
            self.contains(type_name)
        } else {
            false
        }
    }
}
