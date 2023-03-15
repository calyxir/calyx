use calyx_ir::PortComp;

use crate::flatten::{
    flat_ir::prelude::{GuardIdx, PortRef},
    structures::indexed_map::IndexedMap,
};

pub type GuardMap = IndexedMap<GuardIdx, Guard>;

#[derive(Debug)]
pub enum Guard {
    True,
    Or(GuardIdx, GuardIdx),
    And(GuardIdx, GuardIdx),
    Not(GuardIdx),
    Comp(PortComp, PortRef, PortRef),
    Port(PortRef),
}
