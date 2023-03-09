use crate::flatten::flat_ir::{
    component::{AuxillaryComponentInfo, ComponentMap},
    identifier::IdMap,
    prelude::{
        CellDefinition, CellInfo, CombGroupMap, ComponentRef, ControlMap,
        Identifier, LocalPortOffset, LocalRefPortOffset, PortDefinition,
        RefCellDefinition, RefCellInfo, RefPortDefinition,
    },
    wires::{
        core::{AssignmentMap, GroupMap},
        guards::GuardMap,
    },
};

use super::{
    index_trait::IndexRange,
    indexed_map::{AuxillaryMap, IndexedMap},
};

/// The immutable program context for the interpreter. Relevant at simulation
/// time
#[derive(Debug, Default)]
pub struct InterpretationContext {
    /// All assignments in the program
    pub assignments: AssignmentMap,
    /// Component definitions
    pub components: ComponentMap,
    /// All the group definitions
    pub groups: GroupMap,
    /// Comb group definitions
    pub comb_groups: CombGroupMap,
    /// All assignment guards
    pub guards: GuardMap,
    /// Control trees
    pub control: ControlMap,
}

impl InterpretationContext {
    pub fn new() -> Self {
        Default::default()
    }
}

/// Immutable context for the interpreter. Relevant at setup time and during
/// error printing and debugging
pub struct SecondaryContext {
    /// table for mapping strings to identifiers
    pub string_table: IdMap,
    /// non-ref port definitions
    pub local_port_defs: IndexedMap<PortDefinition, Identifier>,
    /// ref-cell ports
    pub ref_port_defs: IndexedMap<RefPortDefinition, Identifier>,
    /// non-ref-cell definitions
    pub local_cell_defs: IndexedMap<CellDefinition, CellInfo>,
    /// ref-cell definitions
    pub ref_cell_defs: IndexedMap<RefCellDefinition, RefCellInfo>,
    /// auxillary information for components
    pub comp_aux_info: AuxillaryMap<ComponentRef, AuxillaryComponentInfo>,
}

impl SecondaryContext {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push_local_port(&mut self, id: Identifier) -> PortDefinition {
        self.local_port_defs.push(id)
    }

    pub fn push_ref_port(&mut self, id: Identifier) -> RefPortDefinition {
        self.ref_port_defs.push(id)
    }

    pub fn push_local_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalPortOffset>,
        parent: ComponentRef,
    ) -> CellDefinition {
        self.local_cell_defs
            .push(CellInfo::new(name, ports, parent))
    }

    pub fn push_ref_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalRefPortOffset>,
        parent: ComponentRef,
    ) -> RefCellDefinition {
        self.ref_cell_defs
            .push(RefCellInfo::new(name, ports, parent))
    }
}

impl Default for SecondaryContext {
    fn default() -> Self {
        Self {
            string_table: IdMap::new(),
            local_port_defs: Default::default(),
            ref_port_defs: Default::default(),
            local_cell_defs: Default::default(),
            ref_cell_defs: Default::default(),
            comp_aux_info: Default::default(),
        }
    }
}
