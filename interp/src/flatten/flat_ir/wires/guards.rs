use calyx::ir::PortComp;

use crate::flatten::structures::{
    environment::PortRef, index_trait::impl_index, indexed_map::IndexedMap,
};

impl_index!(pub GuardIdx);

pub type GuardMap = IndexedMap<Guard, GuardIdx>;

pub enum Guard {
    True,
    Or(GuardIdx, GuardIdx),
    And(GuardIdx, GuardIdx),
    Not(GuardIdx),
    Comp(PortComp, PortRef, PortRef),
    Port(PortRef),
}
