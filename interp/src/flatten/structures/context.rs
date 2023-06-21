use std::ops::Index;

use std::collections::HashMap;

use crate::flatten::flat_ir::{
    cell_prototype::CellPrototype,
    component::{AuxillaryComponentInfo, ComponentCore, ComponentMap},
    identifier::IdMap,
    prelude::{
        Assignment, AssignmentIdx, CellDefinitionIdx, CellInfo, CombGroup,
        CombGroupIdx, CombGroupMap, ComponentIdx, ControlIdx, ControlMap,
        ControlNode, Group, GroupIdx, GuardIdx, Identifier, LocalPortOffset,
        LocalRefPortOffset, ParentIdx, PortDefinitionIdx, PortDefinitionRef,
        PortRef, RefCellDefinitionIdx, RefCellInfo, RefPortDefinitionIdx,
    },
    wires::{
        core::{AssignmentMap, GroupMap},
        guards::{Guard, GuardMap},
    },
};

use super::{
    index_trait::{IndexRange, IndexRef},
    indexed_map::{AuxillaryMap, IndexedMap},
    printer::Printer,
};

use std::ops::AddAssign;

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

impl Index<ComponentIdx> for InterpretationContext {
    type Output = ComponentCore;

    fn index(&self, index: ComponentIdx) -> &Self::Output {
        &self.components[index]
    }
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
    pub comp_aux_info: AuxillaryMap<ComponentIdx, AuxillaryComponentInfo>,
}

impl Index<Identifier> for SecondaryContext {
    type Output = String;

    fn index(&self, index: Identifier) -> &Self::Output {
        self.string_table.lookup_string(&index).unwrap()
    }
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

impl Index<ComponentIdx> for SecondaryContext {
    type Output = AuxillaryComponentInfo;

    fn index(&self, index: ComponentIdx) -> &Self::Output {
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
        parent: ComponentIdx,
        prototype: CellPrototype,
    ) -> CellDefinitionIdx {
        self.local_cell_defs
            .push(CellInfo::new(name, ports, parent, prototype))
    }

    pub fn push_ref_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalRefPortOffset>,
        parent: ComponentIdx,
        prototype: CellPrototype,
    ) -> RefCellDefinitionIdx {
        self.ref_cell_defs
            .push(RefCellInfo::new(name, ports, parent, prototype))
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

/// The full immutable program context for the interpreter.
#[derive(Debug)]
pub struct Context {
    /// Simulation relevant context
    pub primary: InterpretationContext,
    /// Setup/debugging relevant context
    pub secondary: SecondaryContext,
    /// The ID of the entry component for the program (usually called "main")
    /// In general this will be the last component in the program to be
    /// processed and should have the highest index.
    pub entry_point: ComponentIdx,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            primary: Default::default(),
            secondary: Default::default(),
            entry_point: ComponentIdx::new(0),
        }
    }
}

impl From<(InterpretationContext, SecondaryContext)> for Context {
    fn from(
        (primary, secondary): (InterpretationContext, SecondaryContext),
    ) -> Self {
        Self {
            primary,
            secondary,
            entry_point: ComponentIdx::new(0),
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn resolve_id(&self, id: Identifier) -> &String {
        self.secondary.string_table.lookup_string(&id).unwrap()
    }

    pub fn printer(&self) -> Printer {
        Printer::new(self)
    }
}

// Internal helper functions
impl Context {
    #[inline]
    pub(crate) fn lookup_port_definition(
        &self,
        comp: ComponentIdx,
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

    /// This is a wildly inefficient search, only used for debugging right now.
    /// TODO Griffin: if relevant, replace with something more efficient.
    pub(crate) fn find_parent_cell(
        &self,
        comp: ComponentIdx,
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
                        .ports.contains(l));

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
                .find(|c| self.secondary[*c].ports.contains(r))
                .expect("Port does not belong to any ref cell in the given component").into()
            },
        }
    }

    pub fn get_component_size_definitions(
        &self,
        idx: ComponentIdx,
    ) -> SizeDefinitions {
        fn size_definitions_inner(
            ctx: &Context,
            idx: ComponentIdx,
            map: &mut HashMap<ComponentIdx, SizeDefinitions>,
        ) -> SizeDefinitions {
            let (mut ports, mut ref_ports, mut cells, mut ref_cells) =
                (0, 0, 0, 0);
            let aux = &ctx.secondary.comp_aux_info[idx];

            ports += aux.definitions.ports().size();
            ref_ports += aux.definitions.ref_ports().size();
            cells += aux.definitions.cells().size();
            ref_cells += aux.definitions.ref_cells().size();

            let mut out = SizeDefinitions {
                ports,
                ref_ports,
                cells,
                ref_cells,
            };

            for cell in aux.definitions.cells() {
                let proto = &ctx.secondary[cell].prototype;
                if proto.is_component() {
                    let comp = proto.as_component().unwrap();
                    if !map.contains_key(comp) {
                        let result = size_definitions_inner(ctx, *comp, map);
                        map.insert(*comp, result);
                    }
                    out += &map[comp];
                }
            }

            out
        }

        let mut map = HashMap::new();

        size_definitions_inner(self, idx, &mut map)
    }
}

#[derive(Debug)]
pub struct SizeDefinitions {
    pub ports: usize,
    pub cells: usize,
    pub ref_cells: usize,
    pub ref_ports: usize,
}

impl AddAssign<&SizeDefinitions> for SizeDefinitions {
    fn add_assign(&mut self, rhs: &Self) {
        self.ports += rhs.ports;
        self.cells += rhs.cells;
        self.ref_cells += rhs.ref_cells;
        self.ref_ports += rhs.ref_ports;
    }
}
