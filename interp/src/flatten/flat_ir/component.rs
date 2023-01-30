use crate::flatten::structures::{
    index_trait::IndexRange, indexed_map::IndexedMap,
};

use super::{control::structures::ControlIdx, prelude::*};

#[derive(Debug)]
pub struct ComponentCore {
    /// The control program for this component.
    pub control: Option<ControlIdx>,
    /// The set of assignments that are always active.
    pub continuous_assignments: IndexRange<AssignmentIdx>,

    /// True iff component is combinational
    pub is_comb: bool,
}

pub struct AuxillaryComponentInfo {
    /// Name of the component.
    pub name: Identifier,

    /// The input/output signature of this component.
    pub inputs: IndexRange<LocalPortRef>,
    pub outputs: IndexRange<LocalPortRef>,

    /// Groups of assignment wires.
    pub groups: IndexRange<GroupIdx>,
    /// Groups of assignment wires.
    pub comb_groups: IndexRange<CombGroupIdx>,
}

pub type ComponentMap = IndexedMap<ComponentCore, ComponentRef>;
