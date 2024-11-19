use super::{
    super::{
        context::Context, index_trait::IndexRange, indexed_map::IndexedMap,
    },
    assignments::{GroupInterfacePorts, ScheduledAssignments},
    clock::{ClockMap, VectorClock},
    program_counter::{ControlTuple, PcMaps, ProgramCounter, WithEntry},
    traverser::{Path, TraversalError},
};
use crate::{
    configuration::{LoggingConfig, RuntimeConfig},
    errors::{BoxedCiderError, CiderResult, ConflictingAssignments},
    flatten::{
        flat_ir::{
            base::{
                LocalCellOffset, LocalPortOffset, LocalRefCellOffset,
                LocalRefPortOffset,
            },
            cell_prototype::{CellPrototype, SingleWidthType},
            prelude::*,
            wires::guards::Guard,
        },
        primitives::{
            self,
            prim_trait::{RaceDetectionPrimitive, UpdateStatus},
            Primitive,
        },
        structures::{
            context::{LookupName, PortDefinitionInfo},
            environment::{
                program_counter::ControlPoint, traverser::Traverser,
            },
            index_trait::IndexRef,
            thread::{ThreadIdx, ThreadMap},
        },
    },
    logging,
    serialization::{DataDump, MemoryDeclaration, PrintCode},
};
use crate::{
    errors::{RuntimeError, RuntimeResult},
    flatten::structures::environment::wave::WaveWriter,
};
use ahash::HashSet;
use ahash::HashSetExt;
use ahash::{HashMap, HashMapExt};
use baa::{BitVecOps, BitVecValue};
use itertools::Itertools;
use owo_colors::OwoColorize;

use slog::{info, warn, Logger};
use std::fmt::Debug;
use std::fmt::Write;

pub type PortMap = IndexedMap<GlobalPortIdx, PortValue>;

impl PortMap {
    /// Essentially asserts that the port given is undefined, it errors out if
    /// the port is defined and otherwise does nothing
    pub fn write_undef(&mut self, target: GlobalPortIdx) -> RuntimeResult<()> {
        if self[target].is_def() {
            Err(RuntimeError::UndefiningDefinedPort(target).into())
        } else {
            Ok(())
        }
    }

    /// Sets the given index to the given value without checking whether or not
    /// the assignment would conflict with an existing assignment. Should only
    /// be used by cells to set values that may be undefined
    pub fn write_exact_unchecked(
        &mut self,
        target: GlobalPortIdx,
        val: PortValue,
    ) -> UpdateStatus {
        if self[target].is_undef() && val.is_undef()
            || self[target].as_option() == val.as_option()
        {
            UpdateStatus::Unchanged
        } else {
            self[target] = val;
            UpdateStatus::Changed
        }
    }

    /// Sets the given index to undefined without checking whether or not it was
    /// already defined
    #[inline]
    pub fn write_undef_unchecked(&mut self, target: GlobalPortIdx) {
        self[target] = PortValue::new_undef();
    }

    pub fn insert_val(
        &mut self,
        target: GlobalPortIdx,
        val: AssignedValue,
    ) -> Result<UpdateStatus, ConflictingAssignments> {
        match self[target].as_option() {
            // unchanged
            Some(t) if *t == val => Ok(UpdateStatus::Unchanged),
            // conflict
            // TODO: Fix to make the error more helpful
            Some(t)
                if t.has_conflict_with(&val)
                // Assignment & cell values are allowed to override implicit but not the
                // other way around
                    && !(*t.winner() == AssignmentWinner::Implicit) =>
            {
                Err(ConflictingAssignments {
                    target,
                    a1: t.clone(),
                    a2: val,
                })
            }
            // changed
            Some(_) | None => {
                self[target] = PortValue::new(val);
                Ok(UpdateStatus::Changed)
            }
        }
    }

    /// Identical to `insert_val` but returns a `RuntimeError` instead of a
    /// `ConflictingAssignments` error. This should be used inside of primitives
    /// while the latter is used in the general simulation flow.
    pub fn insert_val_general(
        &mut self,
        target: GlobalPortIdx,
        val: AssignedValue,
    ) -> RuntimeResult<UpdateStatus> {
        self.insert_val(target, val)
            .map_err(|e| RuntimeError::ConflictingAssignments(e).into())
    }

    pub fn set_done(
        &mut self,
        target: GlobalPortIdx,
        done_bool: bool,
    ) -> RuntimeResult<UpdateStatus> {
        self.insert_val(
            target,
            AssignedValue::cell_value(if done_bool {
                BitVecValue::tru()
            } else {
                BitVecValue::fals()
            }),
        )
        .map_err(|e| RuntimeError::ConflictingAssignments(e).into())
    }
}

pub(crate) type CellMap = IndexedMap<GlobalCellIdx, CellLedger>;
pub(crate) type RefCellMap =
    IndexedMap<GlobalRefCellIdx, Option<GlobalCellIdx>>;
pub(crate) type RefPortMap =
    IndexedMap<GlobalRefPortIdx, Option<GlobalPortIdx>>;
pub(crate) type AssignmentRange = IndexRange<AssignmentIdx>;

#[derive(Clone)]
pub(crate) struct ComponentLedger {
    pub(crate) index_bases: BaseIndices,
    pub(crate) comp_id: ComponentIdx,
}

impl ComponentLedger {
    /// Convert a relative offset to a global one. Perhaps should take an owned
    /// value rather than a pointer
    pub fn convert_to_global_port(&self, port: &PortRef) -> GlobalPortRef {
        match port {
            PortRef::Local(l) => (&self.index_bases + l).into(),
            PortRef::Ref(r) => (&self.index_bases + r).into(),
        }
    }

    pub fn convert_to_global_cell(&self, cell: &CellRef) -> GlobalCellRef {
        match cell {
            CellRef::Local(l) => (&self.index_bases + l).into(),
            CellRef::Ref(r) => (&self.index_bases + r).into(),
        }
    }
}

/// An enum encapsulating cell functionality. It is either a pointer to a
/// primitive or information about a calyx component instance
pub(crate) enum CellLedger {
    Primitive {
        // wish there was a better option with this one
        cell_dyn: Box<dyn Primitive>,
    },
    RaceDetectionPrimitive {
        cell_dyn: Box<dyn RaceDetectionPrimitive>,
    },
    Component(ComponentLedger),
}

impl Clone for CellLedger {
    fn clone(&self) -> Self {
        match self {
            Self::Primitive { cell_dyn } => Self::Primitive {
                cell_dyn: cell_dyn.clone_boxed(),
            },
            Self::RaceDetectionPrimitive { cell_dyn } => {
                Self::RaceDetectionPrimitive {
                    cell_dyn: cell_dyn.clone_boxed_rd(),
                }
            }
            Self::Component(component_ledger) => {
                Self::Component(component_ledger.clone())
            }
        }
    }
}

impl From<ComponentLedger> for CellLedger {
    fn from(v: ComponentLedger) -> Self {
        Self::Component(v)
    }
}

impl From<Box<dyn RaceDetectionPrimitive>> for CellLedger {
    fn from(cell_dyn: Box<dyn RaceDetectionPrimitive>) -> Self {
        Self::RaceDetectionPrimitive { cell_dyn }
    }
}

impl From<Box<dyn Primitive>> for CellLedger {
    fn from(cell_dyn: Box<dyn Primitive>) -> Self {
        Self::Primitive { cell_dyn }
    }
}

impl CellLedger {
    fn new_comp<C: AsRef<Context> + Clone>(
        idx: ComponentIdx,
        env: &Environment<C>,
    ) -> Self {
        Self::Component(ComponentLedger {
            index_bases: BaseIndices::new(
                env.ports.peek_next_idx(),
                (env.cells.peek_next_idx().index() + 1).into(),
                env.ref_cells.peek_next_idx(),
                env.ref_ports.peek_next_idx(),
            ),
            comp_id: idx,
        })
    }

    pub fn as_comp(&self) -> Option<&ComponentLedger> {
        match self {
            Self::Component(comp) => Some(comp),
            _ => None,
        }
    }

    #[inline]
    pub fn unwrap_comp(&self) -> &ComponentLedger {
        self.as_comp()
            .expect("Unwrapped cell ledger as component but received primitive")
    }

    #[must_use]
    pub fn as_primitive(&self) -> Option<&dyn Primitive> {
        match self {
            Self::Primitive { cell_dyn } => Some(&**cell_dyn),
            Self::RaceDetectionPrimitive { cell_dyn } => {
                Some(cell_dyn.as_primitive())
            }
            _ => None,
        }
    }

    pub fn unwrap_primitive(&self) -> &dyn Primitive {
        self.as_primitive()
            .expect("Unwrapped cell ledger as primitive but received component")
    }
}

impl Debug for CellLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive { .. } => f.debug_struct("Primitive").finish(),
            Self::RaceDetectionPrimitive { .. } => {
                f.debug_struct("RaceDetectionPrimitive").finish()
            }
            Self::Component(ComponentLedger {
                index_bases,
                comp_id,
            }) => f
                .debug_struct("Component")
                .field("index_bases", index_bases)
                .field("comp_id", comp_id)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
struct PinnedPorts {
    map: HashMap<GlobalPortIdx, BitVecValue>,
}

impl PinnedPorts {
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&GlobalPortIdx, &BitVecValue)> + '_ {
        self.map.iter()
    }

    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, port: GlobalPortIdx, val: BitVecValue) {
        self.map.insert(port, val);
    }

    pub fn remove(&mut self, port: GlobalPortIdx) {
        self.map.remove(&port);
    }
}

#[derive(Debug, Clone)]
pub struct Environment<C: AsRef<Context> + Clone> {
    /// A map from global port IDs to their current values.
    ports: PortMap,
    /// A map from global cell IDs to their current state and execution info.
    pub(super) cells: CellMap,
    /// A map from global ref cell IDs to the cell they reference, if any.
    pub(super) ref_cells: RefCellMap,
    /// A map from global ref port IDs to the port they reference, if any.
    pub(super) ref_ports: RefPortMap,

    /// The program counter for the whole program execution.
    pub(super) pc: ProgramCounter,

    pinned_ports: PinnedPorts,

    clocks: ClockMap,
    thread_map: ThreadMap,
    control_ports: HashMap<GlobalPortIdx, u32>,

    /// The immutable context. This is retained for ease of use.
    /// This value should have a cheap clone implementation, such as &Context
    /// or RC<Context>.
    pub(super) ctx: C,

    memory_header: Option<Vec<MemoryDeclaration>>,
    logger: Logger,
}

impl<C: AsRef<Context> + Clone> Environment<C> {
    /// A utility function to get the context from the environment. This is not
    /// suitable for cases in which mutation is required. In such cases, the
    /// context should be accessed directly.
    pub fn ctx(&self) -> &Context {
        self.ctx.as_ref()
    }

    pub fn pc_iter(&self) -> impl Iterator<Item = &ControlPoint> {
        self.pc.iter().map(|(_, x)| x)
    }

    // iterate component instances for DAP
    pub fn iter_compts(
        &self,
    ) -> impl Iterator<Item = (GlobalCellIdx, &String)> + '_ {
        self.cells.iter().filter_map(|(idx, ledge)| match ledge {
            CellLedger::Primitive { .. } => None,
            CellLedger::Component(component_ledger) => {
                Some((idx, self.ctx().lookup_name(component_ledger.comp_id)))
            }
        })
    }
    pub fn iter_cmpt_cells(
        &self,
        cpt: GlobalCellIdx,
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> + '_ {
        // take globalcellid, look up in env to get compt ledger and get base indices
        // w cmpt id, go to context look at ctx.secondary[cmptidx] to get aux info, want cell offset map just keys
        // add local and globel offset, lookup full name and port info
        let ledger = self.cells[cpt].as_comp().unwrap();
        let cells_keys = self.ctx().secondary.comp_aux_info[ledger.comp_id]
            .cell_offset_map
            .keys();
        cells_keys.map(|x| {
            let idx = &ledger.index_bases + x;
            (idx.get_full_name(self), self.ports_helper(idx))
        })
    }

    /// Returns the full name and port list of each cell in the context
    pub fn iter_cells(
        &self,
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> + '_ {
        let env = self;
        let cell_names = self.cells.iter().map(|(idx, _ledger)| {
            (idx.get_full_name(env), self.ports_helper(idx))
        });

        cell_names
        // get parent from cell, if not exist then lookup component ledger get base idxs, go to context and get signature to get ports
        // for cells, same thing but in the cell ledger, subtract child offset from parent offset to get local offset, lookup in cell offset in component info
    }

    //not sure if beneficial to change this to be impl iterator as well
    fn ports_helper(&self, cell: GlobalCellIdx) -> Vec<(String, PortValue)> {
        let parent = self.get_parent_cell_from_cell(cell);
        match parent {
            None => {
                let ports = self.get_ports_from_cell(cell);
                let info = ports
                    .map(|(name, id)| {
                        (
                            (name.lookup_name(self.ctx())).clone(),
                            self.ports[id].clone(),
                        )
                    })
                    .collect_vec();
                info
                // figure out how to unwrap this so i can get the things i need
                // let comp_ledger = self.cells[cell].as_comp().unwrap();
                // let comp_info =
                //     self.ctx().secondary.comp_aux_info.get(comp_ledger.comp_id);
                // let port_ids = comp_info.signature().into_iter().map(|x| {
                //     &self.ctx().secondary.local_port_defs
                //         [comp_info.port_offset_map[x]]
                //         .name
                // });
                // let port_names = port_ids
                //     .map(|x| String::from(x.lookup_name(self.ctx())))
                //     .collect_vec();
                // port_names
            }
            Some(parent_cell) => {
                let ports = self.get_ports_from_cell(parent_cell);
                let info = ports
                    .map(|(name, id)| {
                        (
                            (name.lookup_name(self.ctx())).clone(),
                            self.ports[id].clone(),
                        )
                    })
                    .collect_vec();
                info
            }
        }
    }

    pub fn new(
        ctx: C,
        data_map: Option<DataDump>,
        check_race: bool,
        logging_conf: LoggingConfig,
    ) -> Self {
        let root = ctx.as_ref().entry_point;
        let aux = &ctx.as_ref().secondary[root];

        let mut clocks = IndexedMap::new();
        let root_clock = clocks.push(VectorClock::new());

        let mut env = Self {
            ports: PortMap::with_capacity(aux.port_offset_map.count()),
            cells: CellMap::with_capacity(aux.cell_offset_map.count()),
            ref_cells: RefCellMap::with_capacity(
                aux.ref_cell_offset_map.count(),
            ),
            ref_ports: RefPortMap::with_capacity(
                aux.ref_port_offset_map.count(),
            ),
            pc: ProgramCounter::new_empty(),
            clocks,
            thread_map: ThreadMap::new(root_clock),
            ctx,
            memory_header: None,
            pinned_ports: PinnedPorts::new(),
            control_ports: HashMap::new(),
            logger: logging::initialize_logger(logging_conf),
        };

        let root_node = CellLedger::new_comp(root, &env);
        let root_cell = env.cells.push(root_node);
        env.layout_component(
            root_cell,
            &data_map,
            &mut HashSet::new(),
            check_race,
        );

        let root_thread = ThreadMap::root_thread();
        env.clocks[root_clock].increment(&root_thread);

        // Initialize program counter
        // TODO griffin: Maybe refactor into a separate function
        for (idx, ledger) in env.cells.iter() {
            if let CellLedger::Component(comp) = ledger {
                if let Some(ctrl) =
                    &env.ctx.as_ref().primary[comp.comp_id].control
                {
                    env.pc.vec_mut().push((
                        if comp.comp_id == root {
                            Some(root_thread)
                        } else {
                            None
                        },
                        ControlPoint {
                            comp: idx,
                            control_node_idx: *ctrl,
                        },
                    ))
                }
            }
        }

        if let Some(header) = data_map {
            env.memory_header = Some(header.header.memories);
        }

        env
    }

    /// Internal function used to layout a given component from a cell id
    ///
    /// Layout is handled in the following order:
    /// 1. component signature (input/output)
    /// 2. group hole ports
    /// 3. cells + ports, primitive
    /// 4. sub-components
    /// 5. ref-cells & ports
    fn layout_component(
        &mut self,
        comp: GlobalCellIdx,
        data_map: &Option<DataDump>,
        memories_initialized: &mut HashSet<String>,
        check_race: bool,
    ) {
        // for mutability reasons, see note in `[Environment::new]`
        let ctx = self.ctx.clone();
        let ctx_ref = ctx.as_ref();

        let ComponentLedger {
            index_bases,
            comp_id,
        } = self.cells[comp]
            .as_comp()
            .expect("Called layout component with a non-component cell.");
        let comp_aux = &ctx_ref.secondary[*comp_id];

        // Insert the component's continuous assignments into the program counter, if non-empty
        let cont_assigns =
            self.ctx.as_ref().primary[*comp_id].continuous_assignments;
        if !cont_assigns.is_empty() {
            self.pc.push_continuous_assigns(comp, cont_assigns);
        }

        // first layout the signature
        for sig_port in comp_aux.signature().iter() {
            let def_idx = comp_aux.port_offset_map[sig_port];
            let info = &self.ctx.as_ref().secondary[def_idx];
            let idx = self.ports.push(PortValue::new_undef());

            // the direction attached to the port is reversed for the signature.
            // We only want to add the input ports to the control ports list.
            if !info.is_data && info.direction != calyx_ir::Direction::Input {
                self.control_ports
                    .insert(idx, info.width.try_into().unwrap());
            }
            debug_assert_eq!(index_bases + sig_port, idx);
        }
        // second group ports
        for group_idx in comp_aux.definitions.groups() {
            //go
            let go = self.ports.push(PortValue::new_undef());

            //done
            let done = self.ports.push(PortValue::new_undef());

            // quick sanity check asserts
            let go_actual =
                index_bases + self.ctx.as_ref().primary[group_idx].go;
            let done_actual =
                index_bases + self.ctx.as_ref().primary[group_idx].done;
            // Case 1 - Go defined before done
            if self.ctx.as_ref().primary[group_idx].go
                < self.ctx.as_ref().primary[group_idx].done
            {
                debug_assert_eq!(done, done_actual);
                debug_assert_eq!(go, go_actual);
            }
            // Case 2 - Done defined before go
            else {
                // in this case go is defined after done, so our variable names
                // are backward, but this is not a problem since they are
                // initialized to the same value
                debug_assert_eq!(go, done_actual);
                debug_assert_eq!(done, go_actual);
            }
            self.control_ports.insert(go, 1);
            self.control_ports.insert(done, 1);
        }

        for (cell_off, def_idx) in comp_aux.cell_offset_map.iter() {
            let info = &self.ctx.as_ref().secondary[*def_idx];
            if !info.prototype.is_component() {
                let port_base = self.ports.peek_next_idx();
                for port in info.ports.iter() {
                    let idx = self.ports.push(PortValue::new_undef());
                    debug_assert_eq!(
                        &self.cells[comp].as_comp().unwrap().index_bases + port,
                        idx
                    );
                    let def_idx = comp_aux.port_offset_map[port];
                    let port_info = &self.ctx.as_ref().secondary[def_idx];
                    if !(port_info.direction == calyx_ir::Direction::Output
                        || port_info.is_data && info.is_data)
                    {
                        self.control_ports
                            .insert(idx, port_info.width.try_into().unwrap());
                    }
                }
                let cell_dyn = primitives::build_primitive(
                    info,
                    port_base,
                    self.cells.peek_next_idx(),
                    self.ctx.as_ref(),
                    data_map,
                    memories_initialized,
                    check_race.then_some(&mut self.clocks),
                );
                let cell = self.cells.push(cell_dyn);

                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + cell_off,
                    cell
                );
            } else {
                let child_comp = info.prototype.as_component().unwrap();
                let child_comp = CellLedger::new_comp(*child_comp, self);

                let cell = self.cells.push(child_comp);
                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + cell_off,
                    cell
                );

                // layout sub-component but don't include the data map
                self.layout_component(
                    cell,
                    &None,
                    memories_initialized,
                    check_race,
                );
            }
        }

        if let Some(data) = data_map {
            for dec in data.header.memories.iter() {
                if !memories_initialized.contains(&dec.name) {
                    // TODO griffin: maybe make this an error?
                    warn!(self.logger, "Initialization was provided for memory {} but no such memory exists in the entrypoint component.", dec.name);
                }
            }
        }

        // ref cells and ports are initialized to None
        for (ref_cell, def_idx) in comp_aux.ref_cell_offset_map.iter() {
            let info = &self.ctx.as_ref().secondary[*def_idx];
            for port_idx in info.ports.iter() {
                let port_actual = self.ref_ports.push(None);
                debug_assert_eq!(
                    &self.cells[comp].as_comp().unwrap().index_bases + port_idx,
                    port_actual
                )
            }
            let cell_actual = self.ref_cells.push(None);
            debug_assert_eq!(
                &self.cells[comp].as_comp().unwrap().index_bases + ref_cell,
                cell_actual
            )
        }
    }

    pub fn get_comp_go(&self, comp: GlobalCellIdx) -> GlobalPortIdx {
        let ledger = self.cells[comp]
            .as_comp()
            .expect("Called get_comp_go with a non-component cell.");

        &ledger.index_bases + self.ctx.as_ref().primary[ledger.comp_id].go
    }

    pub fn get_comp_done(&self, comp: GlobalCellIdx) -> GlobalPortIdx {
        let ledger = self.cells[comp]
            .as_comp()
            .expect("Called get_comp_done with a non-component cell.");

        &ledger.index_bases + self.ctx.as_ref().primary[ledger.comp_id].done
    }

    #[inline]
    pub fn get_root_done(&self) -> GlobalPortIdx {
        self.get_comp_done(Self::get_root())
    }

    #[inline]
    pub fn get_root() -> GlobalCellIdx {
        GlobalCellIdx::new(0)
    }

    pub fn is_group_running(&self, group_idx: GroupIdx) -> bool {
        self.get_currently_running_groups().any(|x| x == group_idx)
    }

    pub fn get_currently_running_groups(
        &self,
    ) -> impl Iterator<Item = GroupIdx> + '_ {
        self.pc.iter().filter_map(|(_, point)| {
            let node = &self.ctx.as_ref().primary[point.control_node_idx];
            match node {
                ControlNode::Enable(x) => {
                    let comp_go = self.get_comp_go(point.comp);
                    if self.ports[comp_go].as_bool().unwrap_or_default() {
                        Some(x.group())
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    /// Given a cell idx, return the component definition that this cell is an
    /// instance of. Return None if the cell is not a component instance.
    pub fn get_component_idx(
        &self,
        cell: GlobalCellIdx,
    ) -> Option<ComponentIdx> {
        self.cells[cell].as_comp().map(|x| x.comp_id)
    }

    // ===================== Environment print implementations =====================

    pub fn _print_env(&self) {
        let root_idx = GlobalCellIdx::new(0);
        let mut hierarchy = Vec::new();
        self._print_component(root_idx, &mut hierarchy)
    }

    fn _print_component(
        &self,
        target: GlobalCellIdx,
        hierarchy: &mut Vec<GlobalCellIdx>,
    ) {
        let info = self.cells[target].as_comp().unwrap();
        let comp = &self.ctx.as_ref().secondary[info.comp_id];
        hierarchy.push(target);

        // This funky iterator chain first pulls the first element (the
        // entrypoint) and extracts its name. Subsequent element are pairs of
        // global offsets produced by a staggered iteration, yielding `(root,
        // child)` then `(child, grandchild)` and so on. All the strings are
        // finally collected and concatenated with a `.` separator to produce
        // the fully qualified name prefix for the given component instance.
        let name_prefix = hierarchy
            .first()
            .iter()
            .map(|x| {
                let info = self.cells[**x].as_comp().unwrap();
                let prior_comp = &self.ctx.as_ref().secondary[info.comp_id];
                &self.ctx.as_ref().secondary[prior_comp.name]
            })
            .chain(hierarchy.iter().zip(hierarchy.iter().skip(1)).map(
                |(l, r)| {
                    let info = self.cells[*l].as_comp().unwrap();
                    let prior_comp = &self.ctx.as_ref().secondary[info.comp_id];
                    let local_target = r - (&info.index_bases);

                    let def_idx = &prior_comp.cell_offset_map[local_target];

                    let id = &self.ctx.as_ref().secondary[*def_idx];
                    &self.ctx.as_ref().secondary[id.name]
                },
            ))
            .join(".");

        for (cell_off, def_idx) in comp.cell_offset_map.iter() {
            let definition = &self.ctx.as_ref().secondary[*def_idx];

            println!(
                "{}.{}",
                name_prefix,
                self.ctx.as_ref().secondary[definition.name]
            );
            for port in definition.ports.iter() {
                let definition =
                    &self.ctx.as_ref().secondary[comp.port_offset_map[port]];
                println!(
                    "    {}: {} ({:?})",
                    self.ctx.as_ref().secondary[definition.name],
                    self.ports[&info.index_bases + port],
                    &info.index_bases + port
                );
            }

            let cell_idx = &info.index_bases + cell_off;

            if definition.prototype.is_component() {
                self._print_component(cell_idx, hierarchy);
            } else if self.cells[cell_idx]
                .as_primitive()
                .unwrap()
                .has_serializable_state()
            {
                println!(
                    "    INTERNAL_DATA: {}",
                    serde_json::to_string_pretty(
                        &self.cells[cell_idx]
                            .as_primitive()
                            .unwrap()
                            .serialize(None)
                    )
                    .unwrap()
                )
            }
        }

        hierarchy.pop();
    }

    pub fn _print_env_stats(&self) {
        println!("Environment Stats:");
        println!("  Ports: {}", self.ports.len());
        println!("  Cells: {}", self.cells.len());
        println!("  Ref Cells: {}", self.ref_cells.len());
        println!("  Ref Ports: {}", self.ref_ports.len());
    }

    pub fn print_pc(&self) {
        let current_nodes = self.pc.iter().filter(|(_thread, point)| {
            let node = &self.ctx.as_ref().primary[point.control_node_idx];
            match node {
                ControlNode::Enable(_) | ControlNode::Invoke(_) => {
                    let comp_go = self.get_comp_go(point.comp);
                    self.ports[comp_go].as_bool().unwrap_or_default()
                }

                _ => false,
            }
        });

        let ctx = &self.ctx.as_ref();

        for (_thread, point) in current_nodes {
            let node = &ctx.primary[point.control_node_idx];
            match node {
                ControlNode::Enable(x) => {
                    println!(
                        "{}::{}",
                        self.get_full_name(point.comp),
                        ctx.lookup_name(x.group()).underline()
                    );
                }
                ControlNode::Invoke(x) => {
                    let invoked_name = match x.cell {
                        CellRef::Local(l) => self.get_full_name(
                            &self.cells[point.comp].unwrap_comp().index_bases
                                + l,
                        ),
                        CellRef::Ref(r) => {
                            let ref_global_offset = &self.cells[point.comp]
                                .unwrap_comp()
                                .index_bases
                                + r;
                            let ref_actual =
                                self.ref_cells[ref_global_offset].unwrap();

                            self.get_full_name(ref_actual)
                        }
                    };

                    println!(
                        "{}: invoke {}",
                        self.get_full_name(point.comp),
                        invoked_name.underline()
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn print_pc_string(&self) {
        let ctx = self.ctx.as_ref();
        for node in self.pc_iter() {
            println!(
                "{}: {}",
                self.get_full_name(node.comp),
                node.string_path(ctx)
            );
        }
    }

    fn get_name_from_cell_and_parent(
        &self,
        parent: GlobalCellIdx,
        cell: GlobalCellIdx,
    ) -> Identifier {
        let component = self.cells[parent].unwrap_comp();
        let local_offset = cell - &component.index_bases;

        let def_idx = &self.ctx.as_ref().secondary[component.comp_id]
            .cell_offset_map[local_offset];
        let def_info = &self.ctx.as_ref().secondary[*def_idx];
        def_info.name
    }

    /// Attempt to find the parent cell for a port. If no such cell exists (i.e.
    /// it is a hole port, then it returns None)
    fn get_parent_cell_from_port(
        &self,
        port: PortRef,
        comp: GlobalCellIdx,
    ) -> Option<GlobalCellIdx> {
        let component = self.cells[comp].unwrap_comp();
        let comp_info = &self.ctx.as_ref().secondary[component.comp_id];

        match port {
            PortRef::Local(l) => {
                for (cell_offset, cell_def_idx) in
                    comp_info.cell_offset_map.iter()
                {
                    if self.ctx.as_ref().secondary[*cell_def_idx]
                        .ports
                        .contains(l)
                    {
                        return Some(&component.index_bases + cell_offset);
                    }
                }
            }
            PortRef::Ref(r) => {
                for (cell_offset, cell_def_idx) in
                    comp_info.ref_cell_offset_map.iter()
                {
                    if self.ctx.as_ref().secondary[*cell_def_idx]
                        .ports
                        .contains(r)
                    {
                        let ref_cell_idx = &component.index_bases + cell_offset;
                        return Some(
                            self.ref_cells[ref_cell_idx]
                                .expect("Ref cell has not been instantiated"),
                        );
                    }
                }
            }
        }
        None
    }

    /// returns the path from the root to the given cell, not including the cell
    /// itself. If no such path exists, it returns None.
    fn get_parent_path_from_cell<T: Into<GlobalCellRef>>(
        &self,
        target: T,
    ) -> Option<Vec<GlobalCellIdx>> {
        let target: GlobalCellRef = target.into();

        let root = Self::get_root();
        if target.is_cell() && *target.as_cell().unwrap() == root {
            Some(vec![])
        } else {
            let mut path = vec![root];

            loop {
                // unrwap is safe since there is always at least one entry and
                // the list only grows
                let current = path.last().unwrap();
                let current_comp_ledger =
                    self.cells[*current].as_comp().unwrap();
                let comp_info =
                    &self.ctx.as_ref().secondary[current_comp_ledger.comp_id];

                let possible_relative_offset: CellRef = match target {
                    GlobalCellRef::Cell(target_c) => {
                        (target_c - &current_comp_ledger.index_bases).into()
                    }
                    GlobalCellRef::Ref(target_r) => {
                        (target_r - &current_comp_ledger.index_bases).into()
                    }
                };

                // the target is a direct child
                if match possible_relative_offset {
                    CellRef::Local(l) => comp_info.cell_offset_map.contains(l),
                    CellRef::Ref(r) => {
                        comp_info.ref_cell_offset_map.contains(r)
                    }
                } {
                    return Some(path);
                }
                // the target is a non-direct descendent
                else {
                    match target {
                        GlobalCellRef::Cell(target) => {
                            let mut highest_found = None;
                            for offset in comp_info.cell_offset_map.keys() {
                                let global_offset =
                                    &current_comp_ledger.index_bases + offset;
                                if self.cells[global_offset].as_comp().is_some()
                                    && global_offset < target
                                {
                                    highest_found = Some(global_offset);
                                } else if global_offset > target {
                                    break;
                                }
                            }

                            if let Some(highest_found) = highest_found {
                                path.push(highest_found);
                            } else {
                                return None;
                            }
                        }
                        GlobalCellRef::Ref(r) => {
                            let mut highest_found = None;
                            for offset in comp_info.cell_offset_map.keys() {
                                let global_offset =
                                    &current_comp_ledger.index_bases + offset;

                                if let Some(ledger) =
                                    self.cells[global_offset].as_comp()
                                {
                                    if ledger.index_bases.ref_cell_base <= r {
                                        highest_found = Some(global_offset);
                                    } else {
                                        break;
                                    }
                                }
                            }

                            if let Some(highest_found) = highest_found {
                                path.push(highest_found);
                            } else {
                                return None;
                            }
                        }
                    }
                }
            }
        }
    }

    // this is currently aggressively inefficient but is fine for the moment
    fn get_parent_path_from_port<T: Into<GlobalPortRef>>(
        &self,
        target: T,
    ) -> Option<(Vec<GlobalCellIdx>, Option<GlobalRefCellIdx>)> {
        let target: GlobalPortRef = target.into();

        let mut path = vec![Self::get_root()];

        loop {
            let current = path.last().unwrap();
            let current_ledger = self.cells[*current].as_comp().unwrap();
            let current_info =
                &self.ctx.as_ref().secondary[current_ledger.comp_id];

            if match target {
                GlobalPortRef::Port(p) => {
                    let candidate_offset = p - &current_ledger.index_bases;
                    current_info.port_offset_map.contains(candidate_offset)
                }
                GlobalPortRef::Ref(r) => {
                    let candidate_offset = r - &current_ledger.index_bases;
                    current_info.ref_port_offset_map.contains(candidate_offset)
                }
            }
            // The port is defined in this component
            {
                // first check whether our target is part of the component's
                // signature ports
                if let Some(local) = target.as_port() {
                    let offset = local - &current_ledger.index_bases;
                    if current_info.signature().contains(offset) {
                        return Some((path, None));
                    }
                }

                // now search through the component's cells
                match target {
                    GlobalPortRef::Port(target_idx) => {
                        let target_offset =
                            target_idx - &current_ledger.index_bases;

                        for (offset, def_idx) in
                            current_info.cell_offset_map.iter()
                        {
                            let def = &self.ctx.as_ref().secondary[*def_idx];
                            if def.ports.contains(target_offset) {
                                path.push(&current_ledger.index_bases + offset);
                                return Some((path, None));
                            }
                        }
                        // todo: handle group interface ports
                        return None;
                    }
                    GlobalPortRef::Ref(target_idx) => {
                        let target_offset =
                            target_idx - &current_ledger.index_bases;
                        for (offset, def_idx) in
                            current_info.ref_cell_offset_map.iter()
                        {
                            let def = &self.ctx.as_ref().secondary[*def_idx];
                            if def.ports.contains(target_offset) {
                                return Some((
                                    path,
                                    Some(&current_ledger.index_bases + offset),
                                ));
                            }
                        }
                        return None;
                    }
                }
            }
            // non-direct child
            else {
                let mut highest_found = None;

                for cell_offset in current_info.cell_offset_map.keys() {
                    let cell_offset = &current_ledger.index_bases + cell_offset;
                    if let Some(ledger) = self.cells[cell_offset].as_comp() {
                        match target {
                            GlobalPortRef::Port(target) => {
                                if ledger.index_bases.port_base <= target {
                                    highest_found = Some(cell_offset);
                                } else {
                                    break;
                                }
                            }
                            GlobalPortRef::Ref(target) => {
                                if ledger.index_bases.ref_port_base <= target {
                                    highest_found = Some(cell_offset);
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(highest_found) = highest_found {
                    path.push(highest_found);
                } else {
                    return None;
                }
            }
        }
    }

    pub(super) fn get_parent_cell_from_cell(
        &self,
        cell: GlobalCellIdx,
    ) -> Option<GlobalCellIdx> {
        self.get_parent_path_from_cell(cell)
            .and_then(|x| x.last().copied())
    }

    /// Traverses the given name, and returns the end of the traversal. For
    /// paths with ref cells this will resolve the concrete cell **currently**
    /// pointed to by the ref cell.
    pub fn traverse_name_vec(
        &self,
        name: &[String],
    ) -> Result<Path, TraversalError> {
        assert!(!name.is_empty(), "Name cannot be empty");

        let ctx = self.ctx.as_ref();
        let mut current = Traverser::new();

        if name.len() == 1 && &name[0] == ctx.lookup_name(ctx.entry_point) {
            Ok(Path::Cell(Self::get_root()))
        } else {
            if name.len() != 1 {
                let mut iter = name[0..name.len() - 1].iter();
                if &name[0] == ctx.lookup_name(ctx.entry_point) {
                    // skip the main name
                    iter.next();
                }

                for name in iter {
                    current.next_cell(self, name)?;
                }
            }

            let last = name.last().unwrap();
            current.last_step(self, last)
        }
    }

    pub fn get_ports_from_cell(
        &self,
        cell: GlobalCellIdx,
    ) -> Box<dyn Iterator<Item = (Identifier, GlobalPortIdx)> + '_> {
        if let Some(parent) = self.get_parent_cell_from_cell(cell) {
            let ledger = self.cells[parent].as_comp().unwrap();
            let comp = &self.ctx.as_ref().secondary[ledger.comp_id];
            let cell_offset = cell - &ledger.index_bases;

            Box::new(
                self.ctx.as_ref().secondary[comp.cell_offset_map[cell_offset]]
                    .ports
                    .iter()
                    .map(|x| {
                        (
                            self.ctx.as_ref().secondary
                                [comp.port_offset_map[x]]
                                .name,
                            &ledger.index_bases + x,
                        )
                    }),
            )
        } else {
            let ledger = self.cells[cell].as_comp().unwrap();
            let comp = &self.ctx.as_ref().secondary[ledger.comp_id];
            Box::new(comp.signature().into_iter().map(|x| {
                let def_idx = comp.port_offset_map[x];
                let def = &self.ctx.as_ref().secondary[def_idx];
                (def.name, &ledger.index_bases + x)
            }))
        }
    }

    pub fn get_full_name<N: GetFullName<C>>(&self, nameable: N) -> String {
        nameable.get_full_name(self)
    }

    pub fn format_path(&self, path: &[GlobalCellIdx]) -> String {
        assert!(!path.is_empty(), "Path cannot be empty");
        assert!(path[0] == Self::get_root(), "Path must start with root");

        let root_name =
            self.ctx.as_ref().lookup_name(self.ctx.as_ref().entry_point);

        path.iter().zip(path.iter().skip(1)).fold(
            root_name.clone(),
            |acc, (a, b)| {
                let id = self.get_name_from_cell_and_parent(*a, *b);
                acc + "." + &self.ctx.as_ref().secondary[id]
            },
        )
    }

    /// Lookup the value of a port on the entrypoint component by name. Will
    /// error if the port is not found.
    pub fn lookup_port_from_string<S: AsRef<str>>(
        &self,
        port: S,
    ) -> Option<BitVecValue> {
        // this is not the best way to do this but it's fine for now
        let path = self
            .traverse_name_vec(&[port.as_ref().to_string()])
            .unwrap();
        let path_resolution = path.resolve_path(self).unwrap();
        let idx = path_resolution.as_port().unwrap();

        self.ports[*idx].as_option().map(|x| x.val().clone())
    }

    /// Returns an input port for the entrypoint component. Will error if the
    /// port is not found.
    fn get_root_input_port<S: AsRef<str>>(&self, port: S) -> GlobalPortIdx {
        let string = port.as_ref();

        let root = Self::get_root();

        let ledger = self.cells[root].as_comp().unwrap();
        let mut def_list = self.ctx.as_ref().secondary[ledger.comp_id].inputs();
        let found = def_list.find(|offset| {
            let def_idx = self.ctx.as_ref().secondary[ledger.comp_id].port_offset_map[*offset];
            self.ctx.as_ref().lookup_name(self.ctx.as_ref().secondary[def_idx].name) == string
        }).unwrap_or_else(|| panic!("Could not find port '{string}' in the entrypoint component's input ports"));

        &ledger.index_bases + found
    }

    /// Pins the port with the given name to the given value. This may only be
    /// used for input ports on the entrypoint component (excluding the go port)
    /// and will panic if used otherwise. Intended for external use. Unrelated
    /// to the rust pin.
    pub fn pin_value<S: AsRef<str>>(&mut self, port: S, val: BitVecValue) {
        let port = self.get_root_input_port(port);

        let go = self.get_comp_go(Self::get_root());
        assert!(port != go, "Cannot pin the go port");

        self.pinned_ports.insert(port, val);
    }

    /// Unpins the port with the given name. This may only be
    /// used for input ports on the entrypoint component (excluding the go port)
    /// and will panic if used otherwise. Intended for external use.
    pub fn unpin_value<S: AsRef<str>>(&mut self, port: S) {
        let port = self.get_root_input_port(port);
        self.pinned_ports.remove(port);
    }

    pub fn get_def_info(
        &self,
        comp_idx: ComponentIdx,
        cell: LocalCellOffset,
    ) -> &crate::flatten::flat_ir::base::CellDefinitionInfo<LocalPortOffset>
    {
        let comp = &self.ctx.as_ref().secondary[comp_idx];
        let idx = comp.cell_offset_map[cell];
        &self.ctx.as_ref().secondary[idx]
    }

    pub fn get_def_info_ref(
        &self,
        comp_idx: ComponentIdx,
        cell: LocalRefCellOffset,
    ) -> &crate::flatten::flat_ir::base::CellDefinitionInfo<LocalRefPortOffset>
    {
        let comp = &self.ctx.as_ref().secondary[comp_idx];
        let idx = comp.ref_cell_offset_map[cell];
        &self.ctx.as_ref().secondary[idx]
    }

    pub fn get_port_def_info(
        &self,
        comp_idx: ComponentIdx,
        port: LocalPortOffset,
    ) -> &PortDefinitionInfo {
        let comp = &self.ctx.as_ref().secondary[comp_idx];
        let idx = comp.port_offset_map[port];
        &self.ctx.as_ref().secondary[idx]
    }

    pub fn get_port_def_info_ref(
        &self,
        comp_idx: ComponentIdx,
        port: LocalRefPortOffset,
    ) -> Identifier {
        let comp = &self.ctx.as_ref().secondary[comp_idx];
        let idx = comp.ref_port_offset_map[port];
        self.ctx.as_ref().secondary[idx]
    }
}

/// The core functionality of a simulator. Clonable.
#[derive(Clone)]
pub struct BaseSimulator<C: AsRef<Context> + Clone> {
    env: Environment<C>,
    conf: RuntimeConfig,
}

/// A wrapper struct for the environment that provides the functions used to
/// simulate the actual program.
///
/// This is just to keep the simulation logic under a different namespace than
/// the environment to avoid confusion
pub struct Simulator<C: AsRef<Context> + Clone> {
    base: BaseSimulator<C>,
    wave: Option<WaveWriter>,
}

impl<C: AsRef<Context> + Clone> Simulator<C> {
    pub fn build_simulator(
        ctx: C,
        data_file: &Option<std::path::PathBuf>,
        wave_file: &Option<std::path::PathBuf>,
        runtime_config: RuntimeConfig,
    ) -> Result<Self, BoxedCiderError> {
        let data_dump = data_file
            .as_ref()
            .map(|path| {
                let mut file = std::fs::File::open(path)?;
                DataDump::deserialize(&mut file)
            })
            // flip to a result of an option
            .map_or(Ok(None), |res| res.map(Some))?;
        let env = Environment::new(
            ctx,
            data_dump,
            runtime_config.check_data_race,
            runtime_config.get_logging_config(),
        );
        let wave =
            wave_file.as_ref().map(|p| match WaveWriter::open(p, &env) {
                Ok(w) => w,
                Err(err) => {
                    todo!("deal more gracefully with error: {err:?}")
                }
            });
        Ok(Self {
            base: BaseSimulator::new(env, runtime_config),
            wave,
        })
    }

    pub fn is_done(&self) -> bool {
        self.base.is_done()
    }

    pub fn step(&mut self) -> CiderResult<()> {
        self.base.step()
    }

    pub fn converge(&mut self) -> CiderResult<()> {
        self.base.converge()
    }

    pub fn get_currently_running_groups(
        &self,
    ) -> impl Iterator<Item = GroupIdx> + '_ {
        self.base.get_currently_running_groups()
    }

    pub fn is_group_running(&self, group_idx: GroupIdx) -> bool {
        self.base.is_group_running(group_idx)
    }

    pub fn print_pc(&self) {
        self.base.print_pc();
    }

    pub fn format_cell_state(
        &self,
        cell_idx: GlobalCellIdx,
        print_code: PrintCode,
        name: Option<&str>,
    ) -> Option<String> {
        self.base.format_cell_state(cell_idx, print_code, name)
    }

    pub fn format_cell_ports(
        &self,
        cell_idx: GlobalCellIdx,
        print_code: PrintCode,
        name: Option<&str>,
    ) -> String {
        self.base.format_cell_ports(cell_idx, print_code, name)
    }

    pub fn format_port_value(
        &self,
        port_idx: GlobalPortIdx,
        print_code: PrintCode,
    ) -> String {
        self.base.format_port_value(port_idx, print_code)
    }

    pub fn traverse_name_vec(
        &self,
        name: &[String],
    ) -> Result<Path, TraversalError> {
        self.base.traverse_name_vec(name)
    }

    pub fn get_full_name<N>(&self, nameable: N) -> String
    where
        N: GetFullName<C>,
    {
        self.base.get_full_name(nameable)
    }

    pub(crate) fn env(&self) -> &Environment<C> {
        self.base.env()
    }

    /// Evaluate the entire program
    pub fn run_program(&mut self) -> CiderResult<()> {
        if self.base.conf.debug_logging {
            info!(self.base.env().logger, "Starting program execution");
        }

        match self.base.run_program_inner(self.wave.as_mut()) {
            Ok(_) => {
                if self.base.conf.debug_logging {
                    info!(self.base.env().logger, "Finished program execution");
                }
                Ok(())
            }
            Err(e) => {
                if self.base.conf.debug_logging {
                    slog::error!(
                        self.base.env().logger,
                        "Program execution failed with error: {}",
                        e.red()
                    );
                }
                Err(e)
            }
        }
    }

    pub fn dump_memories(
        &self,
        dump_registers: bool,
        all_mems: bool,
    ) -> DataDump {
        self.base.dump_memories(dump_registers, all_mems)
    }

    pub fn print_pc_string(&self) {
        self.base.print_pc_string()
    }
}

impl<C: AsRef<Context> + Clone> BaseSimulator<C> {
    pub fn new(env: Environment<C>, conf: RuntimeConfig) -> Self {
        Self { env, conf }
    }

    pub(crate) fn env(&self) -> &Environment<C> {
        &self.env
    }

    pub fn _print_env(&self) {
        self.env._print_env()
    }

    #[inline]
    pub fn ctx(&self) -> &Context {
        self.env.ctx.as_ref()
    }

    pub fn _unpack_env(self) -> Environment<C> {
        self.env
    }

    pub fn is_group_running(&self, group_idx: GroupIdx) -> bool {
        self.env.is_group_running(group_idx)
    }

    pub fn get_currently_running_groups(
        &self,
    ) -> impl Iterator<Item = GroupIdx> + '_ {
        self.env.get_currently_running_groups()
    }

    pub fn traverse_name_vec(
        &self,
        name: &[String],
    ) -> Result<Path, TraversalError> {
        self.env.traverse_name_vec(name)
    }

    pub fn get_name_from_cell_and_parent(
        &self,
        parent: GlobalCellIdx,
        cell: GlobalCellIdx,
    ) -> Identifier {
        self.env.get_name_from_cell_and_parent(parent, cell)
    }

    #[inline]
    pub fn get_full_name<N: GetFullName<C>>(&self, nameable: N) -> String {
        self.env.get_full_name(nameable)
    }

    pub fn print_pc(&self) {
        self.env.print_pc()
    }

    pub fn print_pc_string(&self) {
        self.env.print_pc_string()
    }

    /// Pins the port with the given name to the given value. This may only be
    /// used for input ports on the entrypoint component (excluding the go port)
    /// and will panic if used otherwise. Intended for external use.
    pub fn pin_value<S: AsRef<str>>(&mut self, port: S, val: BitVecValue) {
        self.env.pin_value(port, val)
    }

    /// Unpins the port with the given name. This may only be
    /// used for input ports on the entrypoint component (excluding the go port)
    /// and will panic if used otherwise. Intended for external use.
    pub fn unpin_value<S: AsRef<str>>(&mut self, port: S) {
        self.env.unpin_value(port)
    }

    /// Lookup the value of a port on the entrypoint component by name. Will
    /// error if the port is not found.
    pub fn lookup_port_from_string(
        &self,
        port: &String,
    ) -> Option<BitVecValue> {
        self.env.lookup_port_from_string(port)
    }
}

// =========================== simulation functions ===========================
impl<C: AsRef<Context> + Clone> BaseSimulator<C> {
    #[inline]
    fn lookup_global_port_id(&self, port: GlobalPortRef) -> GlobalPortIdx {
        match port {
            GlobalPortRef::Port(p) => p,
            // TODO Griffin: Please make sure this error message is correct with
            // respect to the compiler
            GlobalPortRef::Ref(r) => self.env.ref_ports[r].expect("A ref port is being queried without a supplied ref-cell. This is an error?"),
        }
    }

    #[inline]
    fn lookup_global_cell_id(&self, cell: GlobalCellRef) -> GlobalCellIdx {
        match cell {
            GlobalCellRef::Cell(c) => c,
            // TODO Griffin: Please make sure this error message is correct with
            // respect to the compiler
            GlobalCellRef::Ref(r) => self.env.ref_cells[r].expect("A ref cell is being queried without a supplied ref-cell. This is an error?"),
        }
    }

    #[inline]
    fn get_global_port_idx(
        &self,
        port: &PortRef,
        comp: GlobalCellIdx,
    ) -> GlobalPortIdx {
        let ledger = self.env.cells[comp].unwrap_comp();
        self.lookup_global_port_id(ledger.convert_to_global_port(port))
    }

    #[inline]
    fn get_global_cell_idx(
        &self,
        cell: &CellRef,
        comp: GlobalCellIdx,
    ) -> GlobalCellIdx {
        let ledger = self.env.cells[comp].unwrap_comp();
        self.lookup_global_cell_id(ledger.convert_to_global_cell(cell))
    }

    pub(crate) fn get_root_component(&self) -> &ComponentLedger {
        self.env.cells[Environment::<C>::get_root()]
            .as_comp()
            .unwrap()
    }

    /// Finds the root component of the simulation and sets its go port to high
    fn set_root_go_high(&mut self) {
        let ledger = self.get_root_component();
        let go = &ledger.index_bases
            + self.env.ctx.as_ref().primary[ledger.comp_id].go;
        self.env.ports[go] = PortValue::new_implicit(BitVecValue::tru());
    }

    // may want to make this iterate directly if it turns out that the vec
    // allocation is too expensive in this context
    fn get_assignments(
        &self,
        control_points: &[ControlTuple],
    ) -> Vec<ScheduledAssignments> {
        control_points
            .iter()
            .filter_map(|(thread, node)| {
                match &self.ctx().primary[node.control_node_idx] {
                    ControlNode::Enable(e) => {
                        let group = &self.ctx().primary[e.group()];

                        Some(ScheduledAssignments::new(
                            node.comp,
                            group.assignments,
                            Some(GroupInterfacePorts {
                                go: group.go,
                                done: group.done,
                            }),
                            *thread,
                            false,
                        ))
                    }

                    ControlNode::Invoke(i) => Some(ScheduledAssignments::new(
                        node.comp,
                        i.assignments,
                        None,
                        *thread,
                        false,
                    )),

                    ControlNode::Empty(_) => None,
                    // non-leaf nodes
                    ControlNode::If(_)
                    | ControlNode::While(_)
                    | ControlNode::Repeat(_)
                    | ControlNode::Seq(_)
                    | ControlNode::Par(_) => None,
                }
            })
            .chain(self.env.pc.continuous_assigns().iter().map(|x| {
                ScheduledAssignments::new(x.comp, x.assigns, None, None, true)
            }))
            .chain(self.env.pc.with_map().iter().map(
                |(ctrl_pt, with_entry)| {
                    let assigns =
                        self.ctx().primary[with_entry.group].assignments;
                    ScheduledAssignments::new(
                        ctrl_pt.comp,
                        assigns,
                        None,
                        with_entry.thread,
                        false,
                    )
                },
            ))
            .collect()
    }

    /// A helper function which inserts indicies for the ref cells and ports
    /// used in the invoke statement
    fn initialize_ref_cells(
        &mut self,
        parent_comp: GlobalCellIdx,
        invoke: &Invoke,
    ) {
        if invoke.ref_cells.is_empty() {
            return;
        }

        let parent_ledger = self.env.cells[parent_comp].unwrap_comp();
        let parent_info =
            &self.env.ctx.as_ref().secondary[parent_ledger.comp_id];

        let child_comp = self.get_global_cell_idx(&invoke.cell, parent_comp);
        // this unwrap should never fail because ref-cells can only exist on
        // components, not primitives
        let child_ledger = self.env.cells[child_comp]
            .as_comp()
            .expect("malformed invoke?");
        let child_info = &self.env.ctx.as_ref().secondary[child_ledger.comp_id];

        for (offset, cell_ref) in invoke.ref_cells.iter() {
            // first set the ref cell
            let global_ref_cell_idx = &child_ledger.index_bases + offset;
            let global_actual_cell_idx =
                self.get_global_cell_idx(cell_ref, parent_comp);
            self.env.ref_cells[global_ref_cell_idx] =
                Some(global_actual_cell_idx);

            // then set the ports
            let child_ref_cell_info = &self.env.ctx.as_ref().secondary
                [child_info.ref_cell_offset_map[*offset]];

            let cell_info_idx = parent_info.get_cell_info_idx(*cell_ref);
            match cell_info_idx {
                CellDefinitionRef::Local(l) => {
                    let info = &self.env.ctx.as_ref().secondary[l];
                    assert_eq!(
                        child_ref_cell_info.ports.size(),
                        info.ports.size()
                    );

                    for (dest, source) in
                        child_ref_cell_info.ports.iter().zip(info.ports.iter())
                    {
                        let dest_idx = &child_ledger.index_bases + dest;
                        let source_idx = &parent_ledger.index_bases + source;
                        self.env.ref_ports[dest_idx] = Some(source_idx);
                    }
                }
                CellDefinitionRef::Ref(r) => {
                    let info = &self.env.ctx.as_ref().secondary[r];
                    assert_eq!(
                        child_ref_cell_info.ports.size(),
                        info.ports.size()
                    );

                    for (dest, source) in
                        child_ref_cell_info.ports.iter().zip(info.ports.iter())
                    {
                        let dest_idx = &child_ledger.index_bases + dest;
                        let source_ref_idx =
                            &parent_ledger.index_bases + source;
                        // TODO griffin: Make this error message actually useful
                        let source_idx_actual = self.env.ref_ports
                            [source_ref_idx]
                            .expect("ref port not instantiated, this is a bug");

                        self.env.ref_ports[dest_idx] = Some(source_idx_actual);
                    }
                }
            }
        }
    }

    fn cleanup_ref_cells(
        &mut self,
        parent_comp: GlobalCellIdx,
        invoke: &Invoke,
    ) {
        if invoke.ref_cells.is_empty() {
            return;
        }

        let child_comp = self.get_global_cell_idx(&invoke.cell, parent_comp);
        // this unwrap should never fail because ref-cells can only exist on
        // components, not primitives
        let child_ledger = self.env.cells[child_comp]
            .as_comp()
            .expect("malformed invoke?");
        let child_info = &self.env.ctx.as_ref().secondary[child_ledger.comp_id];

        for (offset, _) in invoke.ref_cells.iter() {
            // first unset the ref cell
            let global_ref_cell_idx = &child_ledger.index_bases + offset;
            self.env.ref_cells[global_ref_cell_idx] = None;

            // then unset the ports
            let child_ref_cell_info = &self.env.ctx.as_ref().secondary
                [child_info.ref_cell_offset_map[*offset]];

            for port in child_ref_cell_info.ports.iter() {
                let port_idx = &child_ledger.index_bases + port;
                self.env.ref_ports[port_idx] = None;
            }
        }
    }

    //
    pub fn converge(&mut self) -> CiderResult<()> {
        self.undef_all_ports();
        self.set_root_go_high();
        // set the pinned values
        for (port, val) in self.env.pinned_ports.iter() {
            self.env.ports[*port] = PortValue::new_implicit(val.clone());
        }

        for (comp, id) in self.env.pc.finished_comps() {
            let done_port = self.env.get_comp_done(*comp);
            let v = PortValue::new_implicit(BitVecValue::tru());
            self.env.ports[done_port] = if self.conf.check_data_race {
                v.with_thread(id.expect("finished comps should have a thread"))
            } else {
                v
            }
        }

        let (vecs, par_map, mut with_map, repeat_map) =
            self.env.pc.take_fields();

        // for mutability reasons, this should be a cheap clone, either an RC in
        // the owned case or a simple reference clone
        let ctx = self.env.ctx.clone();
        let ctx_ref = ctx.as_ref();

        for (thread, node) in vecs.iter() {
            let comp_done = self.env.get_comp_done(node.comp);
            let comp_go = self.env.get_comp_go(node.comp);
            let thread = thread.or_else(|| {
                self.env.ports[comp_go].as_option().and_then(|t| t.thread())
            });

            // if the done is not high & defined, we need to set it to low
            if !self.env.ports[comp_done].as_bool().unwrap_or_default() {
                self.env.ports[comp_done] =
                    PortValue::new_implicit(BitVecValue::fals());
            }

            match &ctx_ref.primary[node.control_node_idx] {
                // actual nodes
                ControlNode::Enable(enable) => {
                    let go_local = ctx_ref.primary[enable.group()].go;
                    let index_bases = &self.env.cells[node.comp]
                        .as_comp()
                        .unwrap()
                        .index_bases;

                    // set go high
                    let go_idx = index_bases + go_local;
                    self.env.ports[go_idx] =
                        PortValue::new_implicit(BitVecValue::tru());
                }
                ControlNode::Invoke(invoke) => {
                    if invoke.comb_group.is_some()
                        && !with_map.contains_key(node)
                    {
                        with_map.insert(
                            node.clone(),
                            WithEntry::new(invoke.comb_group.unwrap(), thread),
                        );
                    }

                    let go = self.get_global_port_idx(&invoke.go, node.comp);
                    self.env.ports[go] =
                        PortValue::new_implicit(BitVecValue::tru())
                            .with_thread_optional(
                                if self.conf.check_data_race {
                                    assert!(thread.is_some());
                                    thread
                                } else {
                                    None
                                },
                            );

                    // TODO griffin: should make this skip initialization if
                    // it's already initialized
                    self.initialize_ref_cells(node.comp, invoke);
                }
                // with nodes
                ControlNode::If(i) => {
                    if i.cond_group().is_some() && !with_map.contains_key(node)
                    {
                        with_map.insert(
                            node.clone(),
                            WithEntry::new(i.cond_group().unwrap(), thread),
                        );
                    }
                }
                ControlNode::While(w) => {
                    if w.cond_group().is_some() && !with_map.contains_key(node)
                    {
                        with_map.insert(
                            node.clone(),
                            WithEntry::new(w.cond_group().unwrap(), thread),
                        );
                    }
                }
                // --
                ControlNode::Empty(_)
                | ControlNode::Seq(_)
                | ControlNode::Par(_)
                | ControlNode::Repeat(_) => {}
            }
        }

        self.env
            .pc
            .restore_fields((vecs, par_map, with_map, repeat_map));

        let assigns_bundle = self.get_assignments(self.env.pc.node_slice());

        self.simulate_combinational(&assigns_bundle)
            .map_err(|e| e.prettify_message(&self.env).into())
    }

    pub fn step(&mut self) -> CiderResult<()> {
        self.converge()?;

        let out: Result<(), BoxedCiderError> = {
            let mut result = Ok(());
            for cell in self.env.cells.values_mut() {
                match cell {
                    CellLedger::Primitive { cell_dyn } => {
                        let res = cell_dyn.exec_cycle(&mut self.env.ports);
                        if res.is_err() {
                            result = Err(res.unwrap_err());
                            break;
                        }
                    }

                    CellLedger::RaceDetectionPrimitive { cell_dyn } => {
                        let res = cell_dyn.exec_cycle_checked(
                            &mut self.env.ports,
                            &mut self.env.clocks,
                            &self.env.thread_map,
                        );
                        if res.is_err() {
                            result = Err(res.unwrap_err());
                            break;
                        }
                    }
                    CellLedger::Component(_) => {}
                }
            }
            result.map_err(|e| e.prettify_message(&self.env).into())
        };

        self.env.pc.clear_finished_comps();

        let mut new_nodes = vec![];
        let (mut vecs, mut par_map, mut with_map, mut repeat_map) =
            self.env.pc.take_fields();

        let mut removed = vec![];

        for (i, node) in vecs.iter_mut().enumerate() {
            let keep_node = self
                .evaluate_control_node(
                    node,
                    &mut new_nodes,
                    (&mut par_map, &mut with_map, &mut repeat_map),
                )
                .map_err(|e| e.prettify_message(&self.env))?;
            if !keep_node {
                removed.push(i);
            }
        }

        for i in removed.into_iter().rev() {
            vecs.swap_remove(i);
        }

        self.env
            .pc
            .restore_fields((vecs, par_map, with_map, repeat_map));

        // insert all the new nodes from the par into the program counter
        self.env.pc.vec_mut().extend(new_nodes);

        out
    }

    fn evaluate_control_node(
        &mut self,
        node: &mut ControlTuple,
        new_nodes: &mut Vec<ControlTuple>,
        maps: PcMaps,
    ) -> RuntimeResult<bool> {
        let (node_thread, node) = node;
        let (par_map, with_map, repeat_map) = maps;
        let comp_go = self.env.get_comp_go(node.comp);
        let comp_done = self.env.get_comp_done(node.comp);

        let thread = node_thread.or_else(|| {
            self.env.ports[comp_go].as_option().and_then(|x| x.thread())
        });

        // mutability trick
        let ctx_clone = self.env.ctx.clone();
        let ctx = ctx_clone.as_ref();

        if !self.env.ports[comp_go].as_bool().unwrap_or_default()
            || self.env.ports[comp_done].as_bool().unwrap_or_default()
        {
            // if the go port is low or the done port is high, we skip the
            // node without doing anything
            return Ok(true);
        }

        // just considering a single node case for the moment
        let retain_bool = match &ctx.primary[node.control_node_idx] {
            ControlNode::Seq(seq) => {
                if !seq.is_empty() {
                    let next = seq.stms()[0];
                    *node = node.new_retain_comp(next);
                    true
                } else {
                    node.mutate_into_next(self.env.ctx.as_ref())
                }
            }
            ControlNode::Par(par) => {
                if par_map.contains_key(node) {
                    let count = par_map.get_mut(node).unwrap();
                    *count -= 1;

                    if *count == 0 {
                        par_map.remove(node);
                        if self.conf.check_data_race {
                            let thread =
                                thread.expect("par nodes should have a thread");

                            let child_clock_idx =
                                self.env.thread_map.unwrap_clock_id(thread);
                            let parent =
                                self.env.thread_map[thread].parent().unwrap();
                            let parent_clock =
                                self.env.thread_map.unwrap_clock_id(parent);
                            let child_clock = std::mem::take(
                                &mut self.env.clocks[child_clock_idx],
                            );
                            self.env.clocks[parent_clock].sync(&child_clock);
                            self.env.clocks[child_clock_idx] = child_clock;
                            assert!(self.env.thread_map[thread]
                                .parent()
                                .is_some());
                            *node_thread = Some(parent);
                            self.env.clocks[parent_clock].increment(&parent);
                        }
                        node.mutate_into_next(self.env.ctx.as_ref())
                    } else {
                        false
                    }
                } else {
                    par_map.insert(
                        node.clone(),
                        par.stms().len().try_into().expect(
                            "More than (2^16 - 1 threads) in a par block. Are you sure this is a good idea?",
                        ),
                    );
                    new_nodes.extend(par.stms().iter().map(|x| {
                        let thread = if self.conf.check_data_race {
                            let thread =
                                thread.expect("par nodes should have a thread");

                            let new_thread_idx: ThreadIdx = *(self
                                .env
                                .pc
                                .lookup_thread(node.comp, thread, *x)
                                .or_insert_with(|| {
                                    let new_clock_idx =
                                        self.env.clocks.new_clock();

                                    self.env
                                        .thread_map
                                        .spawn(thread, new_clock_idx)
                                }));

                            let new_clock_idx = self
                                .env
                                .thread_map
                                .unwrap_clock_id(new_thread_idx);

                            self.env.clocks[new_clock_idx] = self.env.clocks
                                [self.env.thread_map.unwrap_clock_id(thread)]
                            .clone();

                            self.env.clocks[new_clock_idx]
                                .increment(&new_thread_idx);

                            Some(new_thread_idx)
                        } else {
                            None
                        };

                        (thread, node.new_retain_comp(*x))
                    }));

                    if self.conf.check_data_race {
                        let thread =
                            thread.expect("par nodes should have a thread");
                        let clock = self.env.thread_map.unwrap_clock_id(thread);
                        self.env.clocks[clock].increment(&thread);
                    }

                    false
                }
            }
            ControlNode::If(i) => self.handle_if(with_map, node, thread, i)?,
            ControlNode::While(w) => {
                self.handle_while(w, with_map, node, thread)?
            }
            ControlNode::Repeat(rep) => {
                if let Some(count) = repeat_map.get_mut(node) {
                    *count -= 1;
                    if *count == 0 {
                        repeat_map.remove(node);
                        node.mutate_into_next(self.env.ctx.as_ref())
                    } else {
                        *node = node.new_retain_comp(rep.body);
                        true
                    }
                } else {
                    repeat_map.insert(node.clone(), rep.num_repeats);
                    *node = node.new_retain_comp(rep.body);
                    true
                }
            }

            // ===== leaf nodes =====
            ControlNode::Empty(_) => {
                node.mutate_into_next(self.env.ctx.as_ref())
            }
            ControlNode::Enable(e) => {
                let done_local = self.env.ctx.as_ref().primary[e.group()].done;
                let done_idx =
                    &self.env.cells[node.comp].as_comp().unwrap().index_bases
                        + done_local;

                if !self.env.ports[done_idx].as_bool().unwrap_or_default() {
                    true
                } else {
                    // This group has finished running and may be removed
                    // this is somewhat dubious at the moment since it
                    // relies on the fact that the group done port will
                    // still be high since convergence hasn't propagated the
                    // low done signal yet.
                    node.mutate_into_next(self.env.ctx.as_ref())
                }
            }
            ControlNode::Invoke(i) => {
                let done = self.get_global_port_idx(&i.done, node.comp);

                if i.comb_group.is_some() && !with_map.contains_key(node) {
                    with_map.insert(
                        node.clone(),
                        WithEntry::new(i.comb_group.unwrap(), thread),
                    );
                }

                if !self.env.ports[done].as_bool().unwrap_or_default() {
                    true
                } else {
                    self.cleanup_ref_cells(node.comp, i);

                    if i.comb_group.is_some() {
                        with_map.remove(node);
                    }

                    node.mutate_into_next(self.env.ctx.as_ref())
                }
            }
        };

        if !retain_bool && ControlPoint::get_next(node, self.env.ctx.as_ref()).is_none() &&
         // either we are not a par node, or we are the last par node
         (!matches!(&self.env.ctx.as_ref().primary[node.control_node_idx], ControlNode::Par(_)) || !par_map.contains_key(node))
        {
            if self.conf.check_data_race {
                assert!(
                    thread.is_some(),
                    "finished comps should have a thread"
                );
            }

            self.env.pc.set_finshed_comp(node.comp, thread);
            let comp_ledger = self.env.cells[node.comp].unwrap_comp();
            *node = node.new_retain_comp(
                self.env.ctx.as_ref().primary[comp_ledger.comp_id]
                    .control
                    .unwrap(),
            );
            Ok(true)
        } else {
            Ok(retain_bool)
        }
    }

    fn handle_while(
        &mut self,
        w: &While,
        with_map: &mut HashMap<ControlPoint, WithEntry>,
        node: &mut ControlPoint,
        thread: Option<ThreadIdx>,
    ) -> RuntimeResult<bool> {
        let target = GlobalPortRef::from_local(
            w.cond_port(),
            &self.env.cells[node.comp].unwrap_comp().index_bases,
        );

        let idx = match target {
            GlobalPortRef::Port(p) => p,
            GlobalPortRef::Ref(r) => self.env.ref_ports[r]
                .expect("While condition (ref) is undefined"),
        };
        if self.conf.check_data_race {
            if let Some(clocks) = self.env.ports[idx].clocks() {
                let read_clock =
                    self.env.thread_map.unwrap_clock_id(thread.unwrap());
                clocks
                    .check_read(
                        (thread.unwrap(), read_clock),
                        &mut self.env.clocks,
                    )
                    .map_err(|e| {
                        e.add_cell_info(
                            self.env
                                .get_parent_cell_from_port(
                                    w.cond_port(),
                                    node.comp,
                                )
                                .unwrap(),
                        )
                    })?;
            }
        }
        let result = self.env.ports[idx]
            .as_bool()
            .expect("While condition is undefined");

        if result {
            // enter the body
            *node = node.new_retain_comp(w.body());
            Ok(true)
        } else {
            if w.cond_group().is_some() {
                with_map.remove(node);
            }
            // ascend the tree
            Ok(node.mutate_into_next(self.env.ctx.as_ref()))
        }
    }

    fn handle_if(
        &mut self,
        with_map: &mut HashMap<ControlPoint, WithEntry>,
        node: &mut ControlPoint,
        thread: Option<ThreadIdx>,
        i: &If,
    ) -> RuntimeResult<bool> {
        if i.cond_group().is_some() && with_map.get(node).unwrap().entered {
            with_map.remove(node);
            Ok(node.mutate_into_next(self.env.ctx.as_ref()))
        } else {
            if let Some(entry) = with_map.get_mut(node) {
                entry.set_entered()
            }

            let target = GlobalPortRef::from_local(
                i.cond_port(),
                &self.env.cells[node.comp].unwrap_comp().index_bases,
            );
            let idx = match target {
                GlobalPortRef::Port(p) => p,
                GlobalPortRef::Ref(r) => self.env.ref_ports[r]
                    .expect("If condition (ref) is undefined"),
            };

            if self.conf.check_data_race {
                if let Some(clocks) = self.env.ports[idx].clocks() {
                    let read_clock =
                        self.env.thread_map.unwrap_clock_id(thread.unwrap());
                    clocks
                        .check_read(
                            (thread.unwrap(), read_clock),
                            &mut self.env.clocks,
                        )
                        .map_err(|e| {
                            e.add_cell_info(
                                self.env
                                    .get_parent_cell_from_port(
                                        i.cond_port(),
                                        node.comp,
                                    )
                                    .unwrap(),
                            )
                        })?;
                }
            }

            let result = self.env.ports[idx]
                .as_bool()
                .expect("If condition is undefined");

            let target = if result { i.tbranch() } else { i.fbranch() };
            *node = node.new_retain_comp(target);
            Ok(true)
        }
    }

    pub fn is_done(&self) -> bool {
        self.env.ports[self.env.get_root_done()]
            .as_bool()
            .unwrap_or_default()
    }

    pub fn run_program_inner(
        &mut self,
        mut wave: Option<&mut WaveWriter>,
    ) -> Result<(), BoxedCiderError> {
        let mut time = 0;
        while !self.is_done() {
            if let Some(wave) = wave.as_mut() {
                wave.write_values(time, &self.env.ports)?;
            }
            // self.print_pc();
            self.step()?;
            time += 1;
        }
        if let Some(wave) = wave {
            wave.write_values(time, &self.env.ports)?;
        }
        Ok(())
    }

    fn evaluate_guard(
        &self,
        guard: GuardIdx,
        comp: GlobalCellIdx,
    ) -> Option<bool> {
        let guard = &self.ctx().primary[guard];
        match guard {
            Guard::True => Some(true),
            Guard::Or(a, b) => {
                let g1 = self.evaluate_guard(*a, comp)?;
                let g2 = self.evaluate_guard(*b, comp)?;
                Some(g1 || g2)
            }
            Guard::And(a, b) => {
                let g1 = self.evaluate_guard(*a, comp)?;
                let g2 = self.evaluate_guard(*b, comp)?;
                Some(g1 && g2)
            }
            Guard::Not(n) => Some(!self.evaluate_guard(*n, comp)?),
            Guard::Comp(c, a, b) => {
                let comp_v = self.env.cells[comp].unwrap_comp();

                let a = self
                    .lookup_global_port_id(comp_v.convert_to_global_port(a));
                let b = self
                    .lookup_global_port_id(comp_v.convert_to_global_port(b));

                let a_val = self.env.ports[a].val()?;
                let b_val = self.env.ports[b].val()?;
                match c {
                    calyx_ir::PortComp::Eq => a_val == b_val,
                    calyx_ir::PortComp::Neq => a_val != b_val,
                    calyx_ir::PortComp::Gt => a_val.is_greater(b_val),
                    calyx_ir::PortComp::Lt => a_val.is_less(b_val),
                    calyx_ir::PortComp::Geq => a_val.is_greater_or_equal(b_val),
                    calyx_ir::PortComp::Leq => a_val.is_less_or_equal(b_val),
                }
                .into()
            }
            Guard::Port(p) => {
                let comp_v = self.env.cells[comp].unwrap_comp();
                let p_idx = self
                    .lookup_global_port_id(comp_v.convert_to_global_port(p));
                self.env.ports[p_idx].as_bool()
            }
        }
    }

    fn undef_all_ports(&mut self) {
        for (_idx, port_val) in self.env.ports.iter_mut() {
            port_val.set_undef();
        }
    }

    fn simulate_combinational(
        &mut self,
        assigns_bundle: &[ScheduledAssignments],
    ) -> RuntimeResult<()> {
        let mut has_changed = true;
        let mut have_zeroed_control_ports = false;

        if self.conf.debug_logging {
            info!(self.env.logger, "Started combinational convergence");
        }

        while has_changed {
            has_changed = false;

            // evaluate all the assignments and make updates
            for ScheduledAssignments {
                active_cell,
                assignments,
                interface_ports,
                thread,
                is_cont,
            } in assigns_bundle.iter()
            {
                let ledger = self.env.cells[*active_cell].as_comp().unwrap();
                let go = interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.go);
                let done = interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.done);

                let comp_go = self.env.get_comp_go(*active_cell);
                let thread = self.compute_thread(comp_go, thread, go);

                // the go for the group is high
                if go
                    .as_ref()
                    // the group must have its go signal high and the go
                    // signal of the component must also be high
                    .map(|g| {
                        self.env.ports[*g].as_bool().unwrap_or_default()
                            && self.env.ports[comp_go]
                                .as_bool()
                                .unwrap_or_default()
                    })
                    .unwrap_or_else(|| {
                        // if there is no go signal, then we want to run the
                        // continuous assignments but not comb group assignments
                        if *is_cont {
                            true
                        } else {
                            self.env.ports[comp_go]
                                .as_bool()
                                .unwrap_or_default()
                        }
                    })
                {
                    for assign_idx in assignments {
                        let assign = &self.env.ctx.as_ref().primary[assign_idx];

                        // TODO griffin: Come back to this unwrap default later
                        // since we may want to do something different if the guard
                        // does not have a defined value
                        if self
                            .evaluate_guard(assign.guard, *active_cell)
                            .unwrap_or_default()
                        {
                            let port = self
                                .get_global_port_idx(&assign.src, *active_cell);
                            let val = &self.env.ports[port];

                            if self.conf.debug_logging {
                                info!(
                                    self.env.logger,
                                    "Assignment fired in {}: {}\n     wrote {}",
                                    self.env.get_full_name(active_cell),
                                    self.ctx()
                                        .printer()
                                        .print_assignment(
                                            ledger.comp_id,
                                            assign_idx
                                        )
                                        .yellow(),
                                    val.bold()
                                );
                            }

                            if self.conf.check_data_race {
                                if let Some(clocks) = val.clocks() {
                                    // skip checking clocks for continuous assignments
                                    if !is_cont {
                                        if let Some(thread) = thread {
                                            let thread_clock = self
                                                .env
                                                .thread_map
                                                .unwrap_clock_id(thread);

                                            clocks
                                                .check_read(
                                                    (thread, thread_clock),
                                                    &mut self.env.clocks,
                                                )
                                                .map_err(|e| {
                                                    // TODO griffin: find a less hacky way to
                                                    // do this
                                                    e.add_cell_info(
                                                self.env
                                                    .get_parent_cell_from_port(
                                                        assign.src,
                                                        *active_cell,
                                                    )
                                                    .unwrap(),
                                            )
                                                })?;
                                        } else {
                                            panic!("cannot determine thread for non-continuous assignment that touches a checked port");
                                        }
                                    }
                                }
                            }

                            let dest = self
                                .get_global_port_idx(&assign.dst, *active_cell);

                            if let Some(done) = done {
                                if dest != done {
                                    let done_val = &self.env.ports[done];

                                    if done_val.as_bool().unwrap_or(true) {
                                        // skip this assignment when we are done or
                                        // or the done signal is undefined
                                        continue;
                                    }
                                }
                            }

                            if let Some(v) = val.as_option() {
                                let result = self.env.ports.insert_val(
                                    dest,
                                    AssignedValue::new(
                                        v.val().clone(),
                                        (assign_idx, *active_cell),
                                    )
                                    .with_thread_optional(thread),
                                );

                                let changed = match result {
                                    Ok(update) => update,
                                    Err(e) => {
                                        match e.a1.winner() {
                                            AssignmentWinner::Assign(assignment_idx, global_cell_idx) => {
                                                let assign = &self.env.ctx.as_ref().primary[*assignment_idx];
                                                if !self
                                                .evaluate_guard(assign.guard, *global_cell_idx)
                                                .unwrap_or_default() {
                                                    // the prior assignment is
                                                    // no longer valid so we
                                                    // replace it with the new
                                                    // one
                                                    let target = self.get_global_port_idx(&assign.dst, *global_cell_idx);
                                                    self.env.ports[target] = e.a2.into();

                                                    UpdateStatus::Changed
                                                } else {
                                                    return Err(RuntimeError::ConflictingAssignments(e).into());
                                                }
                                            },
                                            _ => return Err(RuntimeError::ConflictingAssignments(e).into()),
                                        }
                                    }
                                };

                                has_changed |= changed.as_bool();
                            }
                            // attempts to undefine a control port that is zero
                            // will be ignored otherwise it is an error
                            // this is a bit of a hack and should be removed in
                            // the long run
                            else if self.env.ports[dest].is_def()
                                && !(self.env.control_ports.contains_key(&dest)
                                    && self.env.ports[dest].is_zero().unwrap())
                            {
                                todo!("Raise an error here since this assignment is undefining things: {}. Port currently has value: {}", self.env.ctx.as_ref().printer().print_assignment(ledger.comp_id, assign_idx), &self.env.ports[dest])
                            }
                        }

                        if self.conf.check_data_race {
                            if let Some(read_ports) = self
                                .env
                                .ctx
                                .as_ref()
                                .primary
                                .guard_read_map
                                .get(assign.guard)
                            {
                                for port in read_ports {
                                    let port_idx = self.get_global_port_idx(
                                        port,
                                        *active_cell,
                                    );
                                    if let Some(clocks) =
                                        self.env.ports[port_idx].clocks()
                                    {
                                        let thread = thread
                                            .expect("cannot determine thread");
                                        let thread_clock = self
                                            .env
                                            .thread_map
                                            .unwrap_clock_id(thread);
                                        clocks
                                            .check_read(
                                                (thread, thread_clock),
                                                &mut self.env.clocks,
                                            )
                                            .map_err(|e| {
                                                // TODO griffin: find a less hacky way to
                                                // do this
                                                e.add_cell_info(
                                                self.env
                                                    .get_parent_cell_from_port(
                                                        *port,
                                                        *active_cell,
                                                    )
                                                    .unwrap(),
                                            )
                                            })?;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Run all the primitives
            let changed: bool = self
                .env
                .cells
                .range()
                .iter()
                .filter_map(|x| match &mut self.env.cells[x] {
                    CellLedger::Primitive { cell_dyn } => {
                        Some(cell_dyn.exec_comb(&mut self.env.ports))
                    }
                    CellLedger::RaceDetectionPrimitive { cell_dyn } => {
                        Some(cell_dyn.exec_comb_checked(
                            &mut self.env.ports,
                            &mut self.env.clocks,
                            &self.env.thread_map,
                        ))
                    }

                    CellLedger::Component(_) => None,
                })
                .fold_ok(UpdateStatus::Unchanged, |has_changed, update| {
                    has_changed | update
                })?
                .as_bool();

            has_changed |= changed;

            // check for undefined done ports. If any remain after we've
            // converged then they should be set to zero and we should continue
            // convergence. Since these ports cannot become undefined again we
            // only need to do this once
            if !has_changed && !have_zeroed_control_ports {
                have_zeroed_control_ports = true;
                for (port, width) in self.env.control_ports.iter() {
                    if self.env.ports[*port].is_undef() {
                        self.env.ports[*port] =
                            PortValue::new_implicit(BitVecValue::zero(*width));
                        has_changed = true;

                        if self.conf.debug_logging {
                            info!(
                                self.env.logger,
                                "Control port {} has been implicitly set to zero",
                                self.env.get_full_name(*port)
                            );
                        }
                    }
                }
            }
        }

        if self.conf.undef_guard_check {
            let mut error_v = vec![];
            for bundle in assigns_bundle.iter() {
                let ledger =
                    self.env.cells[bundle.active_cell].as_comp().unwrap();
                let go = bundle
                    .interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.go);
                let done = bundle
                    .interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.done);

                if !done
                    .and_then(|done| self.env.ports[done].as_bool())
                    .unwrap_or_default()
                    && go
                        .and_then(|go| self.env.ports[go].as_bool())
                        .unwrap_or(true)
                {
                    for assign in bundle.assignments.iter() {
                        let guard_idx = self.ctx().primary[assign].guard;
                        if self
                            .evaluate_guard(guard_idx, bundle.active_cell)
                            .is_none()
                        {
                            let inner_v = self
                                .ctx()
                                .primary
                                .guard_read_map
                                .get(guard_idx)
                                .unwrap()
                                .iter()
                                .filter_map(|p| {
                                    let p = self.get_global_port_idx(
                                        p,
                                        bundle.active_cell,
                                    );
                                    if self.env.ports[p].is_undef() {
                                        Some(p)
                                    } else {
                                        None
                                    }
                                })
                                .collect_vec();

                            error_v.push((bundle.active_cell, assign, inner_v))
                        }
                    }
                }
            }
            if !error_v.is_empty() {
                return Err(RuntimeError::UndefinedGuardError(error_v).into());
            }
        }

        if self.conf.debug_logging {
            info!(self.env.logger, "Finished combinational convergence");
        }

        Ok(())
    }

    /// Attempts to compute the thread id for the given group/component.
    ///
    /// If the given thread is `None`, then the thread id is computed from the
    /// go port for the group. If no such port exists, or it lacks a thread id,
    /// then the thread id is computed from the go port for the component. If
    /// none of these succeed then `None` is returned.
    fn compute_thread(
        &self,
        comp_go: GlobalPortIdx,
        thread: &Option<ThreadIdx>,
        go: Option<GlobalPortIdx>,
    ) -> Option<ThreadIdx> {
        thread.or_else(|| {
            if let Some(go_idx) = go {
                if let Some(go_thread) =
                    self.env.ports[go_idx].as_option().and_then(|a| a.thread())
                {
                    return Some(go_thread);
                }
            }
            self.env.ports[comp_go].as_option().and_then(|x| x.thread())
        })
    }

    /// Dump the current state of the environment as a DataDump
    pub fn dump_memories(
        &self,
        dump_registers: bool,
        all_mems: bool,
    ) -> DataDump {
        let ctx = self.ctx();
        let entrypoint_secondary = &ctx.secondary[ctx.entry_point];

        let mut dump = DataDump::new_empty_with_top_level(
            ctx.resolve_id(entrypoint_secondary.name).clone(),
        );

        let root = self.get_root_component();

        for (offset, idx) in entrypoint_secondary.cell_offset_map.iter() {
            let cell_info = &ctx.secondary[*idx];
            let cell_index = &root.index_bases + offset;
            let name = ctx.resolve_id(cell_info.name).clone();
            match &cell_info.prototype {
                CellPrototype::Memory {
                    width,
                    dims,
                    is_external,
                    ..
                } if *is_external | all_mems => {
                    let declaration =
                        if *is_external && self.env.memory_header.is_some() {
                            if let Some(dec) = self
                                .env
                                .memory_header
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|x| x.name == name)
                            {
                                dec.clone()
                            } else {
                                MemoryDeclaration::new_bitnum(
                                    name,
                                    *width,
                                    dims.as_serializing_dim(),
                                    false,
                                )
                            }
                        } else {
                            MemoryDeclaration::new_bitnum(
                                name,
                                *width,
                                dims.as_serializing_dim(),
                                false,
                            )
                        };

                    dump.push_memory(
                        declaration,
                        self.env.cells[cell_index]
                            .unwrap_primitive()
                            .dump_memory_state()
                            .unwrap(),
                    )
                }
                CellPrototype::SingleWidth {
                    op: SingleWidthType::Reg,
                    width,
                } => {
                    if dump_registers {
                        dump.push_reg(
                            name,
                            *width,
                            self.env.cells[cell_index]
                                .unwrap_primitive()
                                .dump_memory_state()
                                .unwrap(),
                        )
                    }
                }
                _ => (),
            }
        }

        dump
    }

    pub fn get_port_name(
        &self,
        port_idx: GlobalPortIdx,
        parent: GlobalCellIdx,
    ) -> &String {
        let ledger = self.env.cells[parent].as_comp().unwrap();
        let port_offset = port_idx - &ledger.index_bases;
        let def_idx =
            self.ctx().secondary[ledger.comp_id].port_offset_map[port_offset];
        let name = self.ctx().secondary[def_idx].name;
        self.ctx().lookup_name(name)
    }

    pub fn format_port_value(
        &self,
        port_idx: GlobalPortIdx,
        print_code: PrintCode,
    ) -> String {
        self.env.ports[port_idx].format_value(print_code)
    }

    pub fn format_cell_ports(
        &self,
        cell_idx: GlobalCellIdx,
        print_code: PrintCode,
        name: Option<&str>,
    ) -> String {
        let mut buf = String::new();

        if let Some(name_override) = name {
            writeln!(buf, "{name_override}:").unwrap();
        } else {
            writeln!(buf, "{}:", self.get_full_name(cell_idx)).unwrap();
        }
        for (identifier, port_idx) in self.env.get_ports_from_cell(cell_idx) {
            writeln!(
                buf,
                "  {}: {}",
                self.ctx().lookup_name(identifier),
                self.format_port_value(port_idx, print_code)
            )
            .unwrap();
        }

        buf
    }

    pub fn format_cell_state(
        &self,
        cell_idx: GlobalCellIdx,
        print_code: PrintCode,
        name: Option<&str>,
    ) -> Option<String> {
        let cell = self.env.cells[cell_idx].unwrap_primitive();
        let state = cell.serialize(Some(print_code));

        let mut output = String::new();

        if state.has_state() {
            if let Some(name_override) = name {
                write!(output, "{name_override}: ").unwrap();
            } else {
                write!(output, "{}: ", self.get_full_name(cell_idx)).unwrap();
            }

            writeln!(output, "{state}").unwrap();

            Some(output)
        } else {
            None
        }
    }
}

pub trait GetFullName<C: AsRef<Context> + Clone> {
    fn get_full_name(&self, env: &Environment<C>) -> String;
}

impl<C: AsRef<Context> + Clone, T: GetFullName<C>> GetFullName<C> for &T {
    fn get_full_name(&self, env: &Environment<C>) -> String {
        (*self).get_full_name(env)
    }
}

impl<C: AsRef<Context> + Clone> GetFullName<C> for GlobalCellIdx {
    fn get_full_name(&self, env: &Environment<C>) -> String {
        {
            let mut parent_path = env.get_parent_path_from_cell(*self).unwrap();
            parent_path.push(*self);

            env.format_path(&parent_path)
        }
    }
}

impl<C: AsRef<Context> + Clone> GetFullName<C> for GlobalPortIdx {
    fn get_full_name(&self, env: &Environment<C>) -> String {
        if let Some((parent_path, _)) = env.get_parent_path_from_port(*self) {
            let path_str = env.format_path(&parent_path);

            let immediate_parent = parent_path.last().unwrap();
            let comp = if env.cells[*immediate_parent].as_comp().is_some() {
                *immediate_parent
            } else {
                // get second-to-last parent
                parent_path[parent_path.len() - 2]
            };

            let ledger = env.cells[comp].as_comp().unwrap();

            let local_offset = *self - &ledger.index_bases;
            let comp_def = &env.ctx().secondary[ledger.comp_id];
            let port_def_idx = &comp_def.port_offset_map[local_offset];
            let port_def = &env.ctx().secondary[*port_def_idx];
            let name = env.ctx().lookup_name(port_def.name);

            format!("{path_str}.{name}")
        } else {
            // TODO griffin: this is a hack plz fix
            "<unable to get full name>".to_string()
        }
    }
}

impl<C: AsRef<Context> + Clone> GetFullName<C> for GlobalRefCellIdx {
    fn get_full_name(&self, env: &Environment<C>) -> String {
        let parent_path = env.get_parent_path_from_cell(*self).unwrap();
        let path_str = env.format_path(&parent_path);

        let immediate_parent = parent_path.last().unwrap();
        let comp = if env.cells[*immediate_parent].as_comp().is_some() {
            *immediate_parent
        } else {
            // get second-to-last parent
            parent_path[parent_path.len() - 2]
        };

        let ledger = env.cells[comp].as_comp().unwrap();

        let local_offset = *self - &ledger.index_bases;
        let comp_def = &env.ctx().secondary[ledger.comp_id];
        let ref_cell_def_idx = &comp_def.ref_cell_offset_map[local_offset];
        let ref_cell_def = &env.ctx().secondary[*ref_cell_def_idx];
        let name = env.ctx().lookup_name(ref_cell_def.name);

        format!("{path_str}.{name}")
    }
}
