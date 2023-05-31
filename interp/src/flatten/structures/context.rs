use std::ops::Index;

use calyx_ir::PortComp;

use crate::flatten::flat_ir::{
    component::{AuxillaryComponentInfo, ComponentMap},
    identifier::{CanonicalIdentifier, IdMap},
    prelude::{
        Assignment, AssignmentIdx, CellDefinitionIdx, CellInfo, CellRef,
        CombGroup, CombGroupIdx, CombGroupMap, ComponentRef, ControlIdx,
        ControlMap, ControlNode, Group, GroupIdx, GuardIdx, Identifier,
        LocalPortOffset, LocalRefPortOffset, ParentIdx, PortDefinitionIdx,
        PortDefinitionRef, PortRef, RefCellDefinitionIdx, RefCellInfo,
        RefPortDefinitionIdx,
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

impl Index<AssignmentIdx> for InterpretationContext {
    type Output = Assignment;

    fn index(&self, index: AssignmentIdx) -> &Self::Output {
        &self.assignments[index]
    }
}

impl Index<GroupIdx> for InterpretationContext {
    type Output = Group;

    fn index(&self, index: GroupIdx) -> &Self::Output {
        &self.groups[index]
    }
}

impl Index<CombGroupIdx> for InterpretationContext {
    type Output = CombGroup;

    fn index(&self, index: CombGroupIdx) -> &Self::Output {
        &self.comb_groups[index]
    }
}

impl Index<GuardIdx> for InterpretationContext {
    type Output = Guard;

    fn index(&self, index: GuardIdx) -> &Self::Output {
        &self.guards[index]
    }
}

impl Index<ControlIdx> for InterpretationContext {
    type Output = ControlNode;

    fn index(&self, index: ControlIdx) -> &Self::Output {
        &self.control[index]
    }
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
    pub local_port_defs: IndexedMap<PortDefinitionIdx, Identifier>,
    /// ref-cell ports
    pub ref_port_defs: IndexedMap<RefPortDefinitionIdx, Identifier>,
    /// non-ref-cell definitions
    pub local_cell_defs: IndexedMap<CellDefinitionIdx, CellInfo>,
    /// ref-cell definitions
    pub ref_cell_defs: IndexedMap<RefCellDefinitionIdx, RefCellInfo>,
    /// auxillary information for components
    pub comp_aux_info: AuxillaryMap<ComponentRef, AuxillaryComponentInfo>,
}

impl Index<PortDefinitionIdx> for SecondaryContext {
    type Output = Identifier;

    fn index(&self, index: PortDefinitionIdx) -> &Self::Output {
        &self.local_port_defs[index]
    }
}

impl Index<RefPortDefinitionIdx> for SecondaryContext {
    type Output = Identifier;

    fn index(&self, index: RefPortDefinitionIdx) -> &Self::Output {
        &self.ref_port_defs[index]
    }
}

impl Index<CellDefinitionIdx> for SecondaryContext {
    type Output = CellInfo;

    fn index(&self, index: CellDefinitionIdx) -> &Self::Output {
        &self.local_cell_defs[index]
    }
}

impl Index<RefCellDefinitionIdx> for SecondaryContext {
    type Output = RefCellInfo;

    fn index(&self, index: RefCellDefinitionIdx) -> &Self::Output {
        &self.ref_cell_defs[index]
    }
}

impl Index<ComponentRef> for SecondaryContext {
    type Output = AuxillaryComponentInfo;

    fn index(&self, index: ComponentRef) -> &Self::Output {
        &self.comp_aux_info[index]
    }
}

impl SecondaryContext {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push_local_port(&mut self, id: Identifier) -> PortDefinitionIdx {
        self.local_port_defs.push(id)
    }

    pub fn push_ref_port(&mut self, id: Identifier) -> RefPortDefinitionIdx {
        self.ref_port_defs.push(id)
    }

    pub fn push_local_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalPortOffset>,
        parent: ComponentRef,
    ) -> CellDefinitionIdx {
        self.local_cell_defs
            .push(CellInfo::new(name, ports, parent))
    }

    pub fn push_ref_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalRefPortOffset>,
        parent: ComponentRef,
    ) -> RefCellDefinitionIdx {
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
    /// This is a wildly inefficient search, only used for debugging right now.
    /// TODO Griffin: if relevant, replace with something more efficient.
    fn find_parent_cell(
        &self,
        comp: ComponentRef,
        target: PortRef,
    ) -> ParentIdx {
        match target {
            PortRef::Local(l) => {
                if self.secondary[comp].signature.contains(l) {
                    comp.into()
                } else {
                    //I would not recommend looking at this code
                    let port = self.secondary[comp]
                    .definitions
                    .cells()
                    .iter()
                    .find(|c| self.secondary[*c]
                        .ports().contains(l));

                    if let Some(p) = port {
                        p.into()
                    } else {
                         self.secondary[comp].definitions.groups().iter().find(|x| {
                            let grp_info = &self.primary[*x];
                            grp_info.done == l || grp_info.go == l
                        }).unwrap_or_else(|| panic!("Port {:?} does not belong to any normal cell in the given component", l)).into()
                    }


                }
            }
            PortRef::Ref(r) => {
                self.secondary[comp]
                .definitions
                .ref_cells()
                .iter()
                .find(|c| self.secondary[*c].ports().contains(r))
                .expect("Port does not belong to any ref cell in the given component").into()
            },
        }
    }

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
    ) -> CanonicalIdentifier {
        let port = self.lookup_port_definition(comp, target);
        let parent = self.find_parent_cell(comp, target);

        match (port, parent) {
            (PortDefinitionRef::Local(l), ParentIdx::Component(c)) => CanonicalIdentifier::interface_port( self.secondary[c].name, self.secondary[l]),
            (PortDefinitionRef::Local(l), ParentIdx::Cell(c)) => CanonicalIdentifier::cell_port( self.secondary[c].name(), self.secondary[l]),
            (PortDefinitionRef::Local(l), ParentIdx::Group(g)) => CanonicalIdentifier::group_port( self.primary[g].name(), self.secondary[l]),
            (PortDefinitionRef::Ref(rp), ParentIdx::RefCell(rc)) => CanonicalIdentifier::cell_port( self.secondary[rc].name(), self.secondary[rp]),
            _ => unreachable!("Inconsistent port definition and parent. This should never happen"),
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
            dst.format_name(&self.secondary.string_table),
            guard,
            src.format_name(&self.secondary.string_table)
        )
    }
}
