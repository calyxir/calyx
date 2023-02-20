use crate::flatten::flat_ir::{
    component::ComponentMap,
    identifier::IdMap,
    prelude::CombGroupMap,
    wires::{
        core::{AssignmentMap, GroupMap},
        guards::GuardMap,
    },
};

/// The immutable program context for the interpreter.
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
    /// table for mapping strings to identifiers
    pub string_table: IdMap,
}
