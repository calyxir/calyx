use crate::flatten::flat_ir::{
    base::ComponentRef,
    component::ComponentMap,
    identifier::IdMap,
    wires::{
        core::{AssignmentMap, GroupMap},
        guards::GuardMap,
    },
};

use super::indexed_map::IndexedMap;

/// The immutable program context for the interpreter.
#[derive(Debug, Default)]
pub struct InterpretationContext {
    /// All assignments in the program
    pub assignments: AssignmentMap,
    /// Component definitions
    pub components: ComponentMap,
    /// All the group definitions
    pub groups: GroupMap,
    /// All assignment guards
    pub guards: GuardMap,
    /// table for mapping strings to identifiers
    pub string_table: IdMap,
}
