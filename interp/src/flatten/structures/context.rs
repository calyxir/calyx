use calyx_ir::PortComp;

use crate::flatten::flat_ir::{
    component::{AuxillaryComponentInfo, ComponentMap},
    identifier::IdMap,
    prelude::{
        AssignmentIdx, CellDefinition, CellInfo, CombGroupMap, ComponentRef,
        ControlMap, GuardIdx, Identifier, LocalPortOffset, LocalRefPortOffset,
        PortDefinition, PortDefinitionRef, PortRef, RefCellDefinition,
        RefCellInfo, RefPortDefinition,
    },
    wires::{
        core::{AssignmentMap, GroupMap},
        guards::{Guard, GuardMap},
    },
};

use super::{
    index_trait::IndexRange,
    indexed_map::{AuxillaryMap, IndexedMap},
};

/// The full immutable program context for the interpreter.
#[derive(Debug)]
pub struct Context {
    /// Simulation relevant context
    pub primary: InterpretationContext,
    /// Setup/debugging relevant context
    pub secondary: SecondaryContext,
}

impl From<(InterpretationContext, SecondaryContext)> for Context {
    fn from(
        (primary, secondary): (InterpretationContext, SecondaryContext),
    ) -> Self {
        Self { primary, secondary }
    }
}

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

#[derive(Debug)]
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

// Printing and debugging implementations
impl Context {
    #[inline]
    pub fn resolve_id(&self, id: Identifier) -> &String {
        self.secondary.string_table.lookup_string(&id).unwrap()
    }

    #[inline]
    fn lookup_port_definition(
        &self,
        comp: ComponentRef,
        target: PortRef,
    ) -> PortDefinitionRef {
        match target {
            PortRef::Local(l) => {
                self.secondary.comp_aux_info[comp].port_offset_map[l].into()
            }
            PortRef::Ref(r) => {
                self.secondary.comp_aux_info[comp].ref_port_offset_map[r].into()
            }
        }
    }

    #[inline]
    pub fn lookup_id_from_port(
        &self,
        comp: ComponentRef,
        target: PortRef,
    ) -> Identifier {
        let def = self.lookup_port_definition(comp, target);
        match def {
            PortDefinitionRef::Local(l) => self.secondary.local_port_defs[l],
            PortDefinitionRef::Ref(r) => self.secondary.ref_port_defs[r],
        }
    }

    pub fn format_guard(
        &self,
        parent: ComponentRef,
        guard: GuardIdx,
    ) -> String {
        fn op_to_str(op: &PortComp) -> String {
            match op {
                PortComp::Eq => String::from("=="),
                PortComp::Neq => String::from("!="),
                PortComp::Gt => String::from(">"),
                PortComp::Lt => String::from("<"),
                PortComp::Geq => String::from(">="),
                PortComp::Leq => String::from("<="),
            }
        }

        fn inner(ctx: &Context, guard: GuardIdx) -> String {
            match &ctx.primary.guards[guard] {
                Guard::True => String::new(),
                Guard::Or(l, r) => {
                    let l = inner(ctx, *l);
                    let r = inner(ctx, *r);
                    format!("({} | {})", l, r)
                }
                Guard::And(l, r) => {
                    let l = inner(ctx, *l);
                    let r = inner(ctx, *r);
                    format!("({} & {})", l, r)
                }
                Guard::Not(n) => {
                    let n = inner(ctx, *n);
                    format!("(!{})", n)
                }
                Guard::Comp(op, l, r) => {
                    todo!()
                }
                Guard::Port(_) => {
                    todo!()
                }
            }
        }

        let out = inner(self, guard);

        out
    }

    pub fn print_assignment(
        &self,
        parent_comp: ComponentRef,
        target: AssignmentIdx,
    ) -> String {
        let assign = &self.primary.assignments[target];
        let dst = self.lookup_id_from_port(parent_comp, assign.dst);
        let src = self.lookup_id_from_port(parent_comp, assign.src);
        let guard = self.format_guard(parent_comp, assign.guard);
        let guard = if guard.is_empty() {
            guard
        } else {
            format!("{} ? ", guard)
        };

        format!(
            "{} = {}{}",
            self.resolve_id(dst),
            guard,
            self.resolve_id(src)
        )
    }
}
