use crate::flatten::flat_ir::{
    component::{AuxillaryComponentInfo, ComponentMap},
    identifier::IdMap,
    prelude::{
        CombGroupMap, ComponentRef, Identifier, LocalCellInfo, LocalCellRef,
        LocalPortRef, LocalRCellRef, LocalRPortRef, RefCellInfo,
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
    pub port_defs: IndexedMap<LocalPortRef, Identifier>,
    /// ref-cell ports
    pub ref_port_defs: IndexedMap<LocalRPortRef, Identifier>,
    /// non-ref-cell definitions
    pub local_cell_defs: IndexedMap<LocalCellRef, LocalCellInfo>,
    /// ref-cell definitions
    pub ref_cell_defs: IndexedMap<LocalRCellRef, RefCellInfo>,
    /// auxillary information for components
    pub comp_aux_info: AuxillaryMap<ComponentRef, AuxillaryComponentInfo>,
}

impl SecondaryContext {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push_local_id(&mut self, id: Identifier) -> LocalPortRef {
        self.port_defs.push(id)
    }

    pub fn push_ref_id(&mut self, id: Identifier) -> LocalRPortRef {
        self.ref_port_defs.push(id)
    }

    pub fn push_local_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalPortRef>,
    ) -> LocalCellRef {
        self.local_cell_defs.push(LocalCellInfo::new(name, ports))
    }

    pub fn push_ref_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalRPortRef>,
    ) -> LocalRCellRef {
        self.ref_cell_defs.push(RefCellInfo::new(name, ports))
    }
}

impl Default for SecondaryContext {
    fn default() -> Self {
        let mut string_table = IdMap::new();
        string_table.insert("done");
        string_table.insert("go");

        Self {
            string_table,
            port_defs: Default::default(),
            ref_port_defs: Default::default(),
            local_cell_defs: Default::default(),
            ref_cell_defs: Default::default(),
            comp_aux_info: Default::default(),
        }
    }
}
