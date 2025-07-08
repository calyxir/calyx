use std::ops::Index;

use ahash::{HashSet, HashSetExt};
use calyx_frontend::source_info::SourceInfoTable;
use calyx_ir::Direction;
use cider_idx::{
    IndexRef,
    iter::IndexRange,
    maps::{IndexedMap, SecondaryMap, SecondarySparseMap},
};

use crate::{
    errors::{CiderError, CiderResult},
    flatten::flat_ir::{
        cell_prototype::CellPrototype,
        component::{
            AssignmentDefinitionLocation, AuxiliaryComponentInfo, ComponentMap,
            PrimaryComponentInfo,
        },
        identifier::IdMap,
        prelude::{
            Assignment, AssignmentIdx, CellDefinitionIdx, CellInfo, CombGroup,
            CombGroupIdx, CombGroupMap, ComponentIdx, Control, ControlIdx,
            ControlMap, ControlNode, Group, GroupIdx, GuardIdx, Identifier,
            LocalCellOffset, LocalPortOffset, LocalRefCellOffset,
            LocalRefPortOffset, ParentIdx, PortDefinitionIdx,
            PortDefinitionRef, PortRef, RefCellDefinitionIdx, RefCellInfo,
            RefPortDefinitionIdx,
        },
        wires::{
            guards::{Guard, GuardMap},
            structures::{AssignmentMap, GroupMap},
        },
    },
};

use super::printer::Printer;

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
    /// Map from guard to the ports it reads. Might be worth doing some extra
    /// work to make this save memory since empty vecs for True guards is
    /// probably not worth it
    pub guard_read_map: SecondarySparseMap<GuardIdx, Vec<PortRef>>,
    /// Control trees
    pub control: ControlMap,
}

impl Index<ComponentIdx> for InterpretationContext {
    type Output = PrimaryComponentInfo;

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

/// Information about a port definition
#[derive(Debug)]
pub struct PortDefinitionInfo {
    /// The name of the port
    pub name: Identifier,
    /// The width of the port
    pub width: usize,
    /// Whether the port is data
    pub is_data: bool,
    /// The direction of the port
    pub direction: Direction,
}

#[derive(Debug)]
/// Immutable context for the interpreter. Relevant at setup time and during
/// error printing and debugging
pub struct SecondaryContext {
    /// table for mapping strings to identifiers
    pub string_table: IdMap,
    /// non-ref port definitions
    pub local_port_defs: IndexedMap<PortDefinitionIdx, PortDefinitionInfo>,
    /// ref-cell ports
    pub ref_port_defs: IndexedMap<RefPortDefinitionIdx, Identifier>,
    /// non-ref-cell definitions
    pub local_cell_defs: IndexedMap<CellDefinitionIdx, CellInfo>,
    /// ref-cell definitions
    pub ref_cell_defs: IndexedMap<RefCellDefinitionIdx, RefCellInfo>,
    /// auxiliary information for components
    pub comp_aux_info: SecondaryMap<ComponentIdx, AuxiliaryComponentInfo>,
    /// Source Info Table
    pub source_info_table: Option<SourceInfoTable>,
    /// A list of the entangled memories in the program
    pub entangled_mems: Vec<EntangledMemories>,
}

impl Index<Identifier> for SecondaryContext {
    type Output = String;

    fn index(&self, index: Identifier) -> &Self::Output {
        self.string_table.lookup_string(&index).unwrap()
    }
}

impl Index<PortDefinitionIdx> for SecondaryContext {
    type Output = PortDefinitionInfo;

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
    type Output = AuxiliaryComponentInfo;

    fn index(&self, index: ComponentIdx) -> &Self::Output {
        &self.comp_aux_info[index]
    }
}

impl SecondaryContext {
    pub fn new(source_info_table: Option<SourceInfoTable>) -> Self {
        Self {
            string_table: IdMap::new(),
            local_port_defs: Default::default(),
            ref_port_defs: Default::default(),
            local_cell_defs: Default::default(),
            ref_cell_defs: Default::default(),
            comp_aux_info: Default::default(),
            source_info_table,
            entangled_mems: Vec::new(),
        }
    }

    /// Insert a new local port definition into the context and return its index
    pub fn push_local_port(
        &mut self,
        name: Identifier,
        width: usize,
        is_data: bool,
        direction: Direction,
    ) -> PortDefinitionIdx {
        self.local_port_defs.push(PortDefinitionInfo {
            name,
            width,
            is_data,
            direction,
        })
    }

    /// Insert a new reference port definition into the context and return its index
    pub fn push_ref_port(&mut self, id: Identifier) -> RefPortDefinitionIdx {
        self.ref_port_defs.push(id)
    }

    /// Insert a new local cell definition into the context and return its index
    pub fn push_local_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalPortOffset>,
        parent: ComponentIdx,
        prototype: CellPrototype,
        is_data: bool,
    ) -> CellDefinitionIdx {
        self.local_cell_defs
            .push(CellInfo::new(name, ports, parent, prototype, is_data))
    }

    /// Insert a new reference cell definition into the context and return its index
    pub fn push_ref_cell(
        &mut self,
        name: Identifier,
        ports: IndexRange<LocalRefPortOffset>,
        parent: ComponentIdx,
        prototype: CellPrototype,
        is_data: bool,
    ) -> RefCellDefinitionIdx {
        self.ref_cell_defs
            .push(RefCellInfo::new(name, ports, parent, prototype, is_data))
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
            secondary: SecondaryContext::new(None),
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
    /// Create a new empty context
    pub fn new(source_info_table: Option<SourceInfoTable>) -> Self {
        Self {
            primary: Default::default(),
            secondary: SecondaryContext::new(source_info_table),

            entry_point: ComponentIdx::new(0),
        }
    }

    pub fn find_component<F>(&self, query: F) -> Option<ComponentIdx>
    where
        F: Fn(&PrimaryComponentInfo, &AuxiliaryComponentInfo) -> bool,
    {
        self.primary
            .components
            .keys()
            .find(|&comp| query(&self.primary[comp], &self.secondary[comp]))
    }

    /// Resolve the string associated with the given identifier
    #[inline]
    pub fn resolve_id(&self, id: Identifier) -> &String {
        self.secondary.string_table.lookup_string(&id).unwrap()
    }

    /// Create a new printer for the given context
    pub fn printer(&self) -> Printer {
        Printer::new(self)
    }

    /// lookup the port definition for a port offset in a given component. This
    /// will error is the offset is not valid.
    pub fn lookup_port_def(
        &self,
        comp: &ComponentIdx,
        port: LocalPortOffset,
    ) -> &PortDefinitionInfo {
        &self.secondary.local_port_defs
            [self.secondary.comp_aux_info[*comp].port_offset_map[port]]
    }

    /// Lookup the reference port definition for a port offset in a given
    /// component. This will error is the offset is not valid.
    pub fn lookup_ref_port_def(
        &self,
        comp: &ComponentIdx,
        port: LocalRefPortOffset,
    ) -> &Identifier {
        &self.secondary.ref_port_defs
            [self.secondary.comp_aux_info[*comp].ref_port_offset_map[port]]
    }

    /// Lookup the local cell definition for a cell offset in a given component.
    /// This will error is the offset is not valid.
    pub fn lookup_cell_def(
        &self,
        comp: &ComponentIdx,
        cell: LocalCellOffset,
    ) -> &CellInfo {
        &self.secondary.local_cell_defs
            [self.secondary.comp_aux_info[*comp].cell_offset_map[cell]]
    }

    /// Lookup the reference cell definition for a cell offset in a given
    /// component. This will error is the offset is not valid.
    pub fn lookup_ref_cell_def(
        &self,
        comp: &ComponentIdx,
        cell: LocalRefCellOffset,
    ) -> &RefCellInfo {
        &self.secondary.ref_cell_defs
            [self.secondary.comp_aux_info[*comp].ref_cell_offset_map[cell]]
    }

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

    /// Returns the component index with the given name, if such a component exists
    pub fn lookup_comp_by_name(&self, name: &str) -> Option<ComponentIdx> {
        self.find_component(|_, info| {
            info.name.resolve(&self.secondary.string_table) == name
        })
    }

    /// Returns the group index with the given name within the given component, if such a group exists
    pub fn lookup_group_by_name(
        &self,
        name: &str,
        comp: ComponentIdx,
    ) -> Option<GroupIdx> {
        self.secondary[comp]
            .definitions
            .groups()
            .iter()
            .find(|x| self.resolve_id(self.primary[*x].name()) == name)
    }

    /// Return the index of the component which defines the given group
    pub fn get_component_from_group(&self, group: GroupIdx) -> ComponentIdx {
        self.find_component(|_, secondary| {
            secondary.definitions.groups().contains(group)
        })
        .expect("No component defines this group. This should not be possible")
    }

    pub fn lookup_control_definition(
        &self,
        target: ControlIdx,
    ) -> ComponentIdx {
        self.find_component(|_, secondary| {
            secondary.definitions.control().contains(target)
        })
        .expect("No component defines this control node. This should not be possible")
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
                if self.secondary[comp].signature().contains(l) {
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

    /// Lookup the name of the given object. This is used for definitions. For
    /// instances, see [`Environment::get_full_name`](crate::flatten::structures::environment::Environment::get_full_name)
    pub fn lookup_name<T: LookupName>(&self, id: T) -> &String {
        id.lookup_name(self)
    }

    /// Returns information about where an assignment is defined and the
    /// component in which it is defined.
    ///
    /// # Panics
    ///
    /// This function will panic if the given assignment is not defined in any
    /// component.
    pub fn find_assignment_definition(
        &self,
        target: AssignmentIdx,
    ) -> (ComponentIdx, AssignmentDefinitionLocation) {
        for (idx, comp) in self.primary.components.iter() {
            let found = comp.contains_assignment(self, target, idx);
            if let Some(found) = found {
                return (idx, found);
            }
        }
        unreachable!(
            "Assignment '{:?}' does not belong to any component",
            target
        );
    }

    /// Returns the assignment definition information, if it exists. This
    /// requires the component that the assignment is defined in. If the
    /// component is not readily available use
    /// [Self::find_assignment_definition] instead
    pub fn lookup_assignment_definition(
        &self,
        target: AssignmentIdx,
        comp: ComponentIdx,
    ) -> Option<AssignmentDefinitionLocation> {
        self.primary.components[comp].contains_assignment(self, target, comp)
    }

    /// For a given group returns a list of all the control nodes which are
    /// enables of that group
    pub fn find_control_ids_for_group(
        &self,
        group: GroupIdx,
    ) -> Vec<ControlIdx> {
        let comp = self.get_component_from_group(group);
        let comp_ledger = &self.primary.components[comp];
        let mut search_stack = vec![];
        if let Some(id) = comp_ledger.control() {
            search_stack.push(id);
        };

        let mut output = vec![];

        while let Some(current) = search_stack.pop() {
            match &self.primary[current].control {
                Control::Enable(enable) => {
                    if enable.group() == group {
                        output.push(current);
                    }
                }
                Control::Seq(seq) => search_stack.extend(seq.stms()),
                Control::Par(par) => search_stack.extend(par.stms()),
                Control::If(i) => {
                    search_stack.push(i.fbranch());
                    search_stack.push(i.tbranch());
                }
                Control::While(w) => search_stack.push(w.body()),
                Control::Repeat(repeat) => search_stack.push(repeat.body),
                Control::Invoke(_) | Control::Empty(_) => {}
            }
        }

        output
    }

    pub fn string_path(
        &self,
        control_idx: ControlIdx,
        name: &String,
    ) -> String {
        let control_map = &self.primary.control;
        let mut current = control_idx;
        let mut path = vec![control_idx];

        while let Some(parent) = control_map[current].parent {
            path.push(parent);
            current = parent;
        }

        let mut string_path = format!("{name}.");

        // Remove the root
        let mut prev_control_node = &control_map[path.pop().unwrap()].control;

        while let Some(control_idx) = path.pop() {
            // The control_idx should exist in the map, so we shouldn't worry about it
            // exploding. First SearchNode is root, hence "."
            let control_node = &control_map[control_idx].control;

            // we are onto the next iteration and in the body... if Seq or Par is present save their children
            // essentially skip iteration
            match prev_control_node {
                Control::While(_) => {
                    string_path += "-b";
                }
                Control::If(struc) => {
                    let append = if struc.tbranch() == control_idx {
                        "-t"
                    } else {
                        "-f"
                    };

                    string_path += append;
                }
                Control::Par(struc) => {
                    let count =
                        struc.find_child(|&idx| idx == control_idx).unwrap();

                    let control_type = String::from("-") + &count.to_string();
                    string_path = string_path + &control_type;
                }
                Control::Seq(struc) => {
                    let count =
                        struc.find_child(|&idx| idx == control_idx).unwrap();

                    let control_type = String::from("-") + &count.to_string();
                    string_path += &control_type;
                }
                _ => {
                    unreachable!("A terminal node has a child")
                }
            }
            prev_control_node = control_node;
        }
        string_path
    }

    /// Set the
    pub(crate) fn entangle_memories(
        &mut self,
        names: &[String],
    ) -> CiderResult<()> {
        let mut entangled_mems = vec![];
        for mem_grouping in names {
            let mut iter = mem_grouping.split(",").map(|name| {
                let name = name.trim();
                let (comp, mem_name) = if name.contains("::") {
                    let mut part = name.split("::");
                    let comp_name = part.next().unwrap();
                    let mem_name = part.next().unwrap();
                    let comp = self.find_component(|_, info| {
                        self.resolve_id(info.name) == comp_name
                    });
                    let Some(comp) = comp else {
                        return Err(CiderError::generic_error(format!(
                            "No component named '{comp_name}'"
                        )));
                    };
                    (comp, mem_name)
                } else {
                    (self.entry_point, name)
                };

                let mem = self.secondary[comp].definitions.cells().iter().find(
                    |def_idx| {
                        self.resolve_id(self.secondary[*def_idx].name)
                            == mem_name
                    },
                );
                let Some(mem) = mem else {
                    return Err(CiderError::generic_error(format!(
                        "No memory named '{mem_name}'"
                    )));
                };
                Ok(mem)
            });

            let first_cell = iter.next().unwrap()?;

            let Some(cell_prototype) = self.secondary.local_cell_defs
                [first_cell]
                .prototype
                .as_memory()
            else {
                return Err(CiderError::generic_error(format!(
                    "'{}' is not a memory",
                    self.lookup_name(first_cell)
                ))
                .into());
            };

            let mut entangled_grouping = HashSet::new();
            entangled_grouping.insert(first_cell);
            let mut representative: CellDefinitionIdx = first_cell;

            for res in iter {
                let current_cell = res?;

                // must be defined in the same component
                if self.secondary[first_cell].parent
                    != self.secondary[current_cell].parent
                {
                    return Err(CiderError::generic_error(format!(
                        "Entangled memories must be defined in the same component. '{}' and '{}' are defined in '{}' and '{}'",
                        self.lookup_name(first_cell),
                        self.lookup_name(current_cell),
                        self.lookup_name(self.secondary[first_cell].parent),
                        self.lookup_name(self.secondary[current_cell].parent)
                    )).into());
                }

                // cell must be a memory
                let Some(mem_prototype) =
                    self.secondary[current_cell].prototype.as_memory()
                else {
                    return Err(CiderError::generic_error(format!(
                        "'{}' is not a memory",
                        self.lookup_name(current_cell)
                    ))
                    .into());
                };

                // memories need to be the same in shape and type
                if cell_prototype != mem_prototype {
                    return Err(CiderError::generic_error(format!(
                        "Entangled memories must have identical definitions. '{}' and '{}' do not have matching definitions",
                        self.lookup_name(first_cell),
                        self.lookup_name(current_cell),
                    )).into());
                }

                entangled_grouping.insert(current_cell);
                representative = std::cmp::min(representative, current_cell);
            }

            let entangled_group = EntangledMemories {
                group: entangled_grouping,
                representative,
            };

            entangled_mems.push(entangled_group);
        }

        // need to merge any overlapping sets together. The objectively correct
        // thing to do would be to find some Union-Find data structure
        // implementation and force it to work with our keys. If the performance
        // of this thing ever starts to matter we should do that. However, I
        // suspect that given the small number of things we expect to entangle
        // and the fact that this is a one-time operation, that time spent
        // optimizing this extremely bad implementation would be a waste. -G
        let mut merged_entangled_mems = vec![];
        while let Some(mut current) = entangled_mems.pop() {
            loop {
                let initial_len = entangled_mems.len();
                entangled_mems.retain(|group| {
                    if group.overlaps(&current) {
                        current.merge(group);
                        false
                    } else {
                        true
                    }
                });
                if initial_len == entangled_mems.len() {
                    merged_entangled_mems.push(current);
                    break;
                }
            }
        }

        self.secondary.entangled_mems = merged_entangled_mems;
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// A set of cell definitions whose memories should be entangled
pub struct EntangledMemories {
    group: HashSet<CellDefinitionIdx>,
    representative: CellDefinitionIdx,
}

impl EntangledMemories {
    pub fn contains(&self, idx: CellDefinitionIdx) -> bool {
        self.group.contains(&idx)
    }

    pub fn representative(&self) -> CellDefinitionIdx {
        self.representative
    }

    fn merge(&mut self, other: &Self) {
        self.group.extend(&other.group);
        self.representative =
            std::cmp::min(self.representative, other.representative)
    }

    fn overlaps(&self, other: &Self) -> bool {
        !self.group.is_disjoint(&other.group)
    }
}

impl AsRef<Context> for &Context {
    fn as_ref(&self) -> &Context {
        self
    }
}

/// A trait for objects which have a name associated with them in the context.
/// This is used for definitions.
pub trait LookupName {
    /// Lookup the name of the object
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String;
}

impl LookupName for GroupIdx {
    #[inline]
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String {
        ctx.resolve_id(ctx.primary[*self].name())
    }
}

impl LookupName for ComponentIdx {
    #[inline]
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String {
        ctx.resolve_id(ctx.secondary[*self].name)
    }
}

impl LookupName for Identifier {
    #[inline]
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String {
        ctx.resolve_id(*self)
    }
}

impl LookupName for CombGroupIdx {
    #[inline]
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String {
        ctx.resolve_id(ctx.primary[*self].name())
    }
}

impl LookupName for CellDefinitionIdx {
    fn lookup_name<'ctx>(&self, ctx: &'ctx Context) -> &'ctx String {
        ctx.secondary[*self]
            .name
            .resolve(&ctx.secondary.string_table)
    }
}

impl ControlIdx {
    pub fn to_string_path(&self, ctx: &Context) -> String {
        let comp = ctx.lookup_control_definition(*self);
        let comp = comp.lookup_name(ctx);
        ctx.string_path(*self, comp)
    }
}
