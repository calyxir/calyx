use super::{
    assignments::*,
    clock::{ClockMap, ReadSource},
    maps::*,
    program_counter::*,
    traverser::{Path, TraversalError, Traverser},
    wave::WaveWriter,
};

use crate::{
    configuration::{LoggingConfig, RuntimeConfig},
    debugger::{
        self,
        commands::{ParseNodes, ParsePath, PrintTarget},
    },
    errors::*,
    flatten::{
        flat_ir::{
            cell_prototype::{CellPrototype, MemoryPrototype, SingleWidthType},
            indexes::{
                LocalCellOffset, LocalPortOffset, LocalRefCellOffset,
                LocalRefPortOffset,
            },
            prelude::*,
            wires::guards::{Guard, PortComp},
        },
        primitives::{self, prim_trait::UpdateStatus},
        structures::{
            context::{Context, LookupName, PortDefinitionInfo},
            environment::policies::{EvaluationPolicy, PolicyChoice},
            thread::{ThreadIdx, ThreadMap},
        },
        text_utils::Color,
    },
    logging::*,
    serialization::{DataDump, MemoryDeclaration, PrintCode},
};
use std::{
    collections::VecDeque,
    convert::Into,
    fmt::{Debug, Write},
};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use baa::{BitVecOps, BitVecValue};
use calyx_frontend::source_info::PositionId;
use cider_idx::{IndexRef, maps::SecondaryMap};
use delegate::delegate;
use fxhash::{FxHashMap, FxHashSet};
use itertools::Itertools;
use owo_colors::OwoColorize;

#[derive(Debug, Clone)]
pub struct Environment<C: AsRef<Context> + Clone> {
    /// A map from global port IDs to their current values.
    pub(super) ports: PortMap,
    /// A map from global cell IDs to their current state and execution info.
    pub(super) cells: CellMap,
    /// A map from global ref cell IDs to the cell they reference, if any.
    pub(super) ref_cells: RefCellMap,
    /// A map from global ref port IDs to the port they reference, if any.
    pub(super) ref_ports: RefPortMap,
    state_map: MemoryMap,
    /// The program counter for the whole program execution.
    pc: ProgramCounter,
    /// A largely unused map which will force a given port to have a given
    /// value. Mostly meant for use with external tools.
    pinned_ports: PinnedPorts,
    /// Contains all the vector clocks used by the program
    clocks: ClockMap,
    /// Contains information about the threads running within the program.
    thread_map: ThreadMap,
    /// A map containing all the control ports in the program and the width of
    /// the port. Should probably be replaced with a bitset and some extra logic
    /// to lookup widths
    control_ports: FxHashMap<GlobalPortIdx, u32>,
    /// The immutable context. This is retained for ease of use.
    /// This value should have a cheap clone implementation, such as &Context
    /// or RC<Context>.
    ctx: C,
    memory_header: Option<Vec<MemoryDeclaration>>,
    logger: Logger,
    /// Reverse map from ports to the cells they are attached to. Used to
    /// determine which primitives to re=evaluate
    ports_to_cells_map: SecondaryMap<GlobalPortIdx, GlobalCellIdx>,
}

impl<C: AsRef<Context> + Clone> Environment<C> {
    pub fn new(
        ctx: C,
        data_map: Option<DataDump>,
        check_race: bool,
        logging_conf: LoggingConfig,
    ) -> Self {
        /// Internal function used to layout a given component from a cell id
        ///
        /// Layout is handled in the following order:
        /// 1. component signature (input/output)
        /// 2. group hole ports
        /// 3. cells + ports, primitive
        /// 4. sub-components
        /// 5. ref-cells & ports
        fn layout_component<C: AsRef<Context> + Clone>(
            env: &mut Environment<C>,
            ctx: &Context,
            comp: GlobalCellIdx,
            data_map: &Option<DataDump>,
            memories_initialized: &mut HashSet<String>,
            check_race: bool,
        ) {
            let ComponentLedger {
                index_bases,
                comp_id,
            } = env.cells[comp]
                .as_comp()
                .expect("Called layout component with a non-component cell.");
            let comp_aux = &ctx.secondary[*comp_id];

            // Insert the component's continuous assignments into the program counter, if non-empty
            let cont_assigns = ctx.primary[*comp_id].continuous_assignments();
            if !cont_assigns.is_empty() {
                env.pc.push_continuous_assigns(comp, cont_assigns);
            }

            // first layout the signature
            for sig_port in comp_aux.signature().iter() {
                let def_idx = comp_aux.port_offset_map[sig_port];
                let info = &ctx.secondary[def_idx];
                let idx = env.ports.push(PortValue::new_undef());

                // the direction attached to the port is reversed for the signature.
                // We only want to add the input ports to the control ports list.
                if !info.is_data && info.direction != calyx_ir::Direction::Input
                {
                    env.control_ports
                        .insert(idx, info.width.try_into().unwrap());
                }
                debug_assert_eq!(index_bases + sig_port, idx);
            }
            // second group ports
            for group_idx in comp_aux.definitions.groups() {
                //go
                let go = env.ports.push(PortValue::new_undef());
                //done
                let done = env.ports.push(PortValue::new_undef());
                // quick sanity check asserts
                let go_actual = index_bases + ctx.primary[group_idx].go;
                let done_actual = index_bases + ctx.primary[group_idx].done;
                // Case 1 - Go defined before done
                if ctx.primary[group_idx].go < ctx.primary[group_idx].done {
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
                env.control_ports.insert(go, 1);
                env.control_ports.insert(done, 1);
            }

            // ref cells and ports are initialized to None
            for (ref_cell, def_idx) in comp_aux.ref_cell_offset_map.iter() {
                let info = &ctx.secondary[*def_idx];

                for port_idx in info.ports.iter() {
                    let port_actual = env.ref_ports.push(None);
                    debug_assert_eq!(
                        &env.cells[comp].as_comp().unwrap().index_bases
                            + port_idx,
                        port_actual
                    );
                }
                let cell_actual = env.ref_cells.push(None);
                debug_assert_eq!(
                    &env.cells[comp].as_comp().unwrap().index_bases + ref_cell,
                    cell_actual
                )
            }

            let mut memory_entangle_map = HashMap::new();

            for (cell_off, def_idx) in comp_aux.cell_offset_map.iter() {
                let info = &ctx.secondary[*def_idx];
                if !info.prototype.is_component() {
                    let port_base = env.ports.peek_next_idx();
                    for port in info.ports.iter() {
                        let idx = env.ports.push(PortValue::new_undef());
                        debug_assert_eq!(
                            &env.cells[comp].as_comp().unwrap().index_bases
                                + port,
                            idx
                        );
                        let def_idx = comp_aux.port_offset_map[port];
                        let port_info = &ctx.secondary[def_idx];
                        if !(port_info.direction == calyx_ir::Direction::Output
                            || port_info.is_data && info.is_data)
                        {
                            env.control_ports.insert(
                                idx,
                                port_info.width.try_into().unwrap(),
                            );
                        }
                    }
                    let cell_dyn = primitives::build_primitive(
                        *def_idx,
                        port_base,
                        env.cells.peek_next_idx(),
                        ctx,
                        data_map,
                        memories_initialized,
                        check_race.then_some(&mut env.clocks),
                        &mut env.state_map,
                        &mut memory_entangle_map,
                    );
                    let cell = env.cells.push(cell_dyn);

                    debug_assert_eq!(
                        &env.cells[comp].as_comp().unwrap().index_bases
                            + cell_off,
                        cell
                    );
                } else {
                    let child_comp = info.prototype.as_component().unwrap();
                    let child_comp = CellLedger::new_comp(*child_comp, env);

                    let cell = env.cells.push(child_comp);
                    debug_assert_eq!(
                        &env.cells[comp].as_comp().unwrap().index_bases
                            + cell_off,
                        cell
                    );

                    // layout sub-component but don't include the data map
                    layout_component(
                        env,
                        ctx,
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
                        warn!(
                            env.logger,
                            "Initialization was provided for memory {} but no such memory exists in the entrypoint component.",
                            dec.name
                        );
                    }
                }
            }
        }

        let root = ctx.as_ref().entry_point;
        let aux = &ctx.as_ref().secondary[root];

        let mut clocks = ClockMap::new();
        let root_clock = clocks.new_clock();
        let continuous_clock = clocks.new_clock();

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
            ctx: ctx.clone(),
            clocks,
            thread_map: ThreadMap::new(root_clock, continuous_clock),
            memory_header: None,
            control_ports: FxHashMap::new(),
            logger: initialize_logger(logging_conf),
            ports_to_cells_map: SecondaryMap::new_with_default(0.into()),
            pinned_ports: PinnedPorts::new(),
            state_map: MemoryMap::new(),
        };

        let ctx = ctx.as_ref();

        let root_node = CellLedger::new_comp(root, &env);
        let root_cell = env.cells.push(root_node);
        layout_component(
            &mut env,
            ctx,
            root_cell,
            &data_map,
            &mut HashSet::new(),
            check_race,
        );

        let root_thread = ThreadMap::root_thread();
        env.clocks[root_clock].increment(&root_thread);
        env.clocks[continuous_clock].increment(&ThreadMap::continuous_thread());

        // Initialize program counter
        // TODO griffin: Maybe refactor into a separate function
        for (idx, ledger) in env.cells.iter() {
            if let CellLedger::Component(comp) = ledger {
                let comp_info = &ctx.primary[comp.comp_id];
                if !comp_info.is_comb()
                    && let Some(ctrl) = comp_info.as_standard().unwrap().control
                    {
                        env.pc.vec_mut().push(ProgramPointer::new_active(
                            (comp.comp_id == root).then_some(root_thread),
                            ControlPoint {
                                comp: idx,
                                control_node_idx: ctrl,
                            },
                        ))
                    }
            }
        }

        if let Some(header) = data_map {
            env.memory_header = Some(header.header.memories);
        }

        env.ports_to_cells_map =
            SecondaryMap::capacity_with_default(0.into(), env.ports.len());

        for (cell, ledger) in env.cells.iter() {
            if let Some(comp_ledger) = ledger.as_comp() {
                for port in comp_ledger.signature_ports(ctx) {
                    env.ports_to_cells_map.insert(port, cell);
                }
            } else {
                let cell_dyn = ledger.as_primitive().unwrap();
                for port in cell_dyn.get_ports().iter_all() {
                    env.ports_to_cells_map.insert(port, cell);
                }
            }
        }
        env
    }

    /// A utility function to get the context from the environment. This is not
    /// suitable for cases in which mutation is required. In such cases, the
    /// context should be accessed directly.
    pub fn ctx(&self) -> &Context {
        self.ctx.as_ref()
    }

    pub fn pc_iter(&self) -> impl Iterator<Item = &ControlPoint> {
        self.pc.iter().map(ProgramPointer::control_point)
    }

    /// Method that returns an iterator over all component instances in the debugger
    /// Used for Cider-DAP extension
    pub fn iter_compts(
        &self,
    ) -> impl Iterator<Item = (GlobalCellIdx, &String)> {
        self.cells.iter().filter_map(|(idx, ledge)| match ledge {
            CellLedger::Primitive { .. } => None,
            CellLedger::Component(component_ledger) => {
                Some((idx, self.ctx().lookup_name(component_ledger.comp_id)))
            }
            CellLedger::RaceDetectionPrimitive { .. } => None, //what this
        })
    }
    /// Method that returns an iterator over all cells in component cpt
    /// Used for Cider-DAP extension
    pub fn iter_cmpt_cells(
        &self,
        cpt: GlobalCellIdx,
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> {
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
    ) -> impl Iterator<Item = (String, Vec<(String, PortValue)>)> {
        let env = self;

        self.cells.iter().map(|(idx, _ledger)| {
            (idx.get_full_name(env), self.ports_helper(idx))
        })
        // get parent from cell, if not exist then lookup component ledger get base idxs, go to context and get signature to get ports
        // for cells, same thing but in the cell ledger, subtract child offset from parent offset to get local offset, lookup in cell offset in component info
    }

    //not sure if beneficial to change this to be impl iterator as well
    fn ports_helper(&self, cell: GlobalCellIdx) -> Vec<(String, PortValue)> {
        let parent = self.get_parent_cell_from_cell(cell);
        match parent {
            None => {
                let ports = self.get_ports_from_cell(cell);

                ports
                    .map(|(name, id)| {
                        (
                            (name.lookup_name(self.ctx())).clone(),
                            self.ports[id].clone(),
                        )
                    })
                    .collect_vec()
            }
            Some(parent_cell) => {
                let ports = self.get_ports_from_cell(parent_cell);

                ports
                    .map(|(name, id)| {
                        (
                            (name.lookup_name(self.ctx())).clone(),
                            self.ports[id].clone(),
                        )
                    })
                    .collect_vec()
            }
        }
    }

    pub fn get_comp_go(&self, comp: GlobalCellIdx) -> Option<GlobalPortIdx> {
        let ledger = self.cells[comp]
            .as_comp()
            .expect("Called get_comp_go with a non-component cell.");

        let go_port = self.ctx.as_ref().primary[ledger.comp_id]
            .as_standard()
            .map(|x| x.go);

        go_port.map(|go| &ledger.index_bases + go)
    }

    pub fn unwrap_comp_go(&self, comp: GlobalCellIdx) -> GlobalPortIdx {
        self.get_comp_go(comp).unwrap()
    }

    pub fn comp_go_as_bool(&self, comp: GlobalCellIdx) -> bool {
        let go = self.unwrap_comp_go(comp);
        self.ports[go].as_bool().unwrap_or_default()
    }

    pub fn get_comp_done(&self, comp: GlobalCellIdx) -> Option<GlobalPortIdx> {
        let ledger = self.cells[comp]
            .as_comp()
            .expect("Called get_comp_done with a non-component cell.");

        let done_port = self.ctx.as_ref().primary[ledger.comp_id]
            .as_standard()
            .map(|x| x.done);
        done_port.map(|done| &ledger.index_bases + done)
    }

    pub fn unwrap_comp_done(&self, comp: GlobalCellIdx) -> GlobalPortIdx {
        self.get_comp_done(comp).unwrap()
    }

    #[inline]
    pub fn get_root_done(&self) -> GlobalPortIdx {
        self.get_comp_done(Self::get_root()).unwrap()
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
    ) -> impl Iterator<Item = GroupIdx> {
        self.pc.iter().filter_map(|point| {
            let node = &self.ctx.as_ref().primary[point.control_idx()].control;
            match node {
                Control::Enable(x) => {
                    let comp_go = self.get_comp_go(point.component()).unwrap();
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

    pub fn is_control_running(&self, control_idx: ControlIdx) -> bool {
        self.get_currently_running_nodes().any(|x| x == control_idx)
    }

    pub fn get_currently_running_nodes(
        &self,
    ) -> impl Iterator<Item = ControlIdx> {
        self.pc.iter().filter_map(|point| {
            let comp_go = self.get_comp_go(point.component()).unwrap();
            if self.ports[comp_go].as_bool().unwrap_or_default() {
                Some(point.control_idx())
            } else {
                None
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

    pub fn print_pc(&self) -> String {
        let mut out = String::new();
        let current_nodes = self.pc.iter().filter(|point| {
            let node = &self.ctx.as_ref().primary[point.control_idx()].control;
            match node {
                Control::Enable(_) | Control::Invoke(_) => {
                    let comp_go = self.unwrap_comp_go(point.component());
                    self.ports[comp_go].as_bool().unwrap_or_default()
                }

                _ => false,
            }
        });

        let ctx = &self.ctx.as_ref();

        for point in current_nodes {
            let node = &ctx.primary[point.control_idx()].control;
            match node {
                Control::Enable(x) => {
                    let go = &self.cells[point.component()]
                        .unwrap_comp()
                        .index_bases
                        + self.ctx().primary[x.group()].go;
                    writeln!(
                        out,
                        "{}::{}{}",
                        self.get_full_name(point.component()),
                        ctx.lookup_name(x.group()).stylize_name(),
                        if self.ports[go].as_bool().unwrap_or_default() {
                            ""
                        } else {
                            " [done]"
                        }
                    )
                    .expect("couldn't write string");
                }
                Control::Invoke(x) => {
                    let invoked_name = match x.cell {
                        CellRef::Local(l) => self.get_full_name(
                            &self.cells[point.component()]
                                .unwrap_comp()
                                .index_bases
                                + l,
                        ),
                        CellRef::Ref(r) => {
                            let ref_global_offset = &self.cells
                                [point.component()]
                            .unwrap_comp()
                            .index_bases
                                + r;
                            let ref_actual =
                                self.ref_cells[ref_global_offset].unwrap();

                            self.get_full_name(ref_actual)
                        }
                    };

                    writeln!(
                        out,
                        "{}: invoke {}",
                        self.get_full_name(point.component()),
                        invoked_name.stylize_name()
                    )
                    .expect("couldn't write string");
                }
                _ => unreachable!(),
            }
        }
        out
    }

    /// Returns the controlidx of the last node in the given path and component idx
    pub fn path_idx(
        &self,
        component: ComponentIdx,
        path: ParsePath,
    ) -> ControlIdx {
        let path_nodes = path.get_path();
        let ctx = self.ctx();

        let component_map = &ctx.primary.components;
        let control_map = &ctx.primary.control;

        // Get nodes
        let component_node = component_map.get(component).unwrap();

        let mut control_id = component_node.control().unwrap();

        let mut control_node = &control_map[control_id].control;
        for parse_node in path_nodes {
            match parse_node {
                ParseNodes::Body => match control_node {
                    Control::While(while_struct) => {
                        control_id = while_struct.body();
                    }
                    Control::Repeat(repeat_struct) => {
                        control_id = repeat_struct.body;
                    }
                    _ => {
                        // TODO: Dont want to crash if invalid path, return result type w/ error malformed
                        panic!();
                    }
                },
                ParseNodes::If(branch) => match control_node {
                    Control::If(if_struct) => {
                        control_id = if branch {
                            if_struct.tbranch()
                        } else {
                            if_struct.fbranch()
                        };
                    }
                    _ => {
                        panic!();
                    }
                },
                ParseNodes::Offset(child) => match control_node {
                    Control::Par(par_struct) => {
                        let children = par_struct.stms();
                        control_id = children[child as usize];
                    }
                    Control::Seq(seq_struct) => {
                        let children = seq_struct.stms();
                        control_id = children[child as usize];
                    }
                    _ => {
                        // Do nothing! use same control_id!
                    }
                },
            }
            control_node = &control_map[control_id].control;
        }
        control_id
    }

    pub fn print_pc_string(&self) {
        let ctx = self.ctx.as_ref();
        for node in self.pc_iter() {
            let ledger = self.cells.get(node.comp).unwrap();
            let comp_ledger = ledger.as_comp().unwrap();
            let component = comp_ledger.comp_id;
            let string_path = ctx
                .string_path(node.control_node_idx, component.lookup_name(ctx));
            println!("{}: {}", self.get_full_name(node.comp), string_path);

            let path =
                debugger::commands::path_parser::parse_path(&string_path)
                    .unwrap();

            let control_idx = self.path_idx(component, path);

            debug_assert_eq!(control_idx, node.control_node_idx);
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
    fn _get_parent_cell_from_port(
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

        let go = self.unwrap_comp_go(Self::get_root());
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
    ) -> &crate::flatten::flat_ir::indexes::CellDefinitionInfo<LocalPortOffset>
    {
        let comp = &self.ctx.as_ref().secondary[comp_idx];
        let idx = comp.cell_offset_map[cell];
        &self.ctx.as_ref().secondary[idx]
    }

    pub fn get_def_info_ref(
        &self,
        comp_idx: ComponentIdx,
        cell: LocalRefCellOffset,
    ) -> &crate::flatten::flat_ir::indexes::CellDefinitionInfo<LocalRefPortOffset>
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

    pub fn iter_positions(&self) -> impl Iterator<Item = PositionId> {
        self.pc
            .iter()
            .filter_map(|ctrl_point| {
                let node = &self.ctx().primary[ctrl_point.control_idx()];

                if self.comp_go_as_bool(ctrl_point.component()) {
                    Some(node.positions())
                } else {
                    None
                }
            })
            .flatten()
    }

    pub fn get_parent_from_port(
        &self,
        port: GlobalPortIdx,
    ) -> Option<GlobalCellIdx> {
        self.cells
            .iter()
            .find(|(_, ledger)| match ledger {
                CellLedger::Primitive { cell_dyn } => {
                    cell_dyn.get_ports().contains(port)
                }
                CellLedger::RaceDetectionPrimitive { cell_dyn } => {
                    cell_dyn.get_ports().contains(port)
                }
                CellLedger::Component(component_ledger) => {
                    component_ledger.signature_ports(self.ctx()).contains(port)
                }
            })
            .map(|(idx, _)| idx)
    }

    pub fn get_prototype(&self, cell: GlobalCellIdx) -> &CellPrototype {
        assert!(self.cells[cell].as_primitive().is_some());
        let parent = self.get_parent_cell_from_cell(cell).unwrap();
        let comp = self.cells[parent].unwrap_comp();
        let comp_idx = comp.comp_id;
        let local_idx = cell - &comp.index_bases;

        let def_idx = self.ctx().secondary[comp_idx].cell_offset_map[local_idx];
        &self.ctx().secondary[def_idx].prototype
    }
}

enum ControlNodeEval {
    Reprocess,
    Stop { retain_node: bool },
}

impl ControlNodeEval {
    fn stop(retain_node: bool) -> Self {
        ControlNodeEval::Stop { retain_node }
    }
}

/// A wrapper struct for the environment that provides the functions used to
/// simulate the actual program.
///
/// This is just to keep the simulation logic under a different namespace than
/// the environment to avoid confusion
pub struct BaseSimulator<C: AsRef<Context> + Clone> {
    env: Environment<C>,
    conf: RuntimeConfig,
    policy: Box<dyn EvaluationPolicy>,
}

impl<C: AsRef<Context> + Clone> Clone for BaseSimulator<C> {
    fn clone(&self) -> Self {
        Self {
            env: self.env.clone(),
            conf: self.conf,
            policy: self.policy.box_clone(),
        }
    }
}

impl<C: AsRef<Context> + Clone> BaseSimulator<C> {
    pub(crate) fn new(
        env: Environment<C>,
        conf: RuntimeConfig,
        policy: Box<dyn EvaluationPolicy>,
    ) -> Self {
        Self { env, conf, policy }
    }

    pub(crate) fn env(&self) -> &Environment<C> {
        &self.env
    }

    delegate! {
            to self.env {
                pub fn ctx(&self) -> &Context;
                pub fn is_group_running(&self, group_idx: GroupIdx) -> bool;
                pub fn is_control_running(&self, control_idx: ControlIdx) -> bool;

                pub fn get_currently_running_groups(
                    &self,
                ) -> impl Iterator<Item = GroupIdx>;

                pub fn traverse_name_vec(
                    &self,
                    name: &[String],
                ) -> Result<Path, TraversalError>;

                pub fn get_name_from_cell_and_parent(
                    &self,
                    parent: GlobalCellIdx,
                    cell: GlobalCellIdx,
                ) -> Identifier;

                #[inline]
                pub fn get_full_name<N: GetFullName<C>>(&self, nameable: N) -> String;
                pub fn print_pc(&self) -> String;
                pub fn print_pc_string(&self);
                /// Pins the port with the given name to the given value. This may only be
                /// used for input ports on the entrypoint component (excluding the go port)
                /// and will panic if used otherwise. Intended for external use.
                pub fn pin_value<S: AsRef<str>>(&mut self, port: S, val: BitVecValue);
                /// Unpins the port with the given name. This may only be
                /// used for input ports on the entrypoint component (excluding the go port)
                /// and will panic if used otherwise. Intended for external use.
                pub fn unpin_value<S: AsRef<str>>(&mut self, port: S);
                /// Lookup the value of a port on the entrypoint component by name. Will
                /// error if the port is not found.
                pub fn lookup_port_from_string(
                    &self,
                    port: &String,
                ) -> Option<BitVecValue>;

                pub fn get_currently_running_nodes(
                    &self,
                ) -> impl Iterator<Item = ControlIdx>;
        }
    }

    /// Return an iterator over all cells that are in scope for the current set
    /// of assignments.
    pub fn iter_active_cells(&self) -> impl Iterator<Item = GlobalCellIdx> {
        let assigns_bundle = self.get_assignments(self.env.pc.node_slice());

        let mut referenced_cells: HashSet<GlobalCellIdx> = HashSet::new();

        for ScheduledAssignments {
            active_cell,
            assignments,
            interface_ports,
            assign_type,
            ..
        } in assigns_bundle
        {
            referenced_cells.insert(active_cell);
            let ledger = self.env.cells[active_cell].as_comp().unwrap();
            let group_go =
                interface_ports.as_ref().map(|x| &ledger.index_bases + x.go);
            let comp_go = self.env.get_comp_go(active_cell);

            if self.is_assign_bundle_active(assign_type, group_go, comp_go) {
                for assign in assignments.iter() {
                    let assignment = &self.ctx().primary[assign];
                    let src =
                        self.get_global_port_idx(&assignment.src, active_cell);
                    let dst =
                        self.get_global_port_idx(&assignment.dst, active_cell);

                    if let Some(src) = self.env.get_parent_from_port(src) {
                        referenced_cells.insert(src);
                    }

                    if let Some(dst) = self.env.get_parent_from_port(dst) {
                        referenced_cells.insert(dst);
                    }

                    if let Some(ports) =
                        self.ctx().primary.guard_read_map.get(assignment.guard)
                    {
                        for port in ports.iter() {
                            let port =
                                self.get_global_port_idx(port, active_cell);
                            if let Some(parent) =
                                self.env.get_parent_from_port(port)
                            {
                                referenced_cells.insert(parent);
                            }
                        }
                    }
                }
            }
        }

        referenced_cells.into_iter().sorted()
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
            + self.env.ctx.as_ref().primary[ledger.comp_id]
                .unwrap_standard()
                .go;
        self.env.ports[go] = PortValue::new_implicit(BitVecValue::new_true());
    }

    // may want to make this iterate directly if it turns out that the vec
    // allocation is too expensive in this context
    fn get_assignments(
        &self,
        control_points: &[ProgramPointer],
    ) -> Vec<ScheduledAssignments> {
        let mut skiplist = HashSet::new();
        let mut additional_groups = VecDeque::new();

        let mut out: Vec<ScheduledAssignments> = control_points
            .iter()
            .filter_map(|point| {
                if !point.is_enabled() {
                    return None;
                }

                match &self.ctx().primary[point.control_idx()].control {
                    Control::Enable(e) => {
                        let group = &self.ctx().primary[e.group()];

                        skiplist.insert((point.component(), e.group()));
                        additional_groups.extend(
                            group
                                .structural_enables
                                .iter()
                                .copied()
                                .map(|grp| (point.component(), grp)),
                        );

                        Some(ScheduledAssignments::new_control(
                            point.component(),
                            group.assignments,
                            Some(GroupInterfacePorts {
                                go: group.go,
                                done: group.done,
                            }),
                            point.thread(),
                        ))
                    }

                    Control::Invoke(i) => {
                        Some(ScheduledAssignments::new_control(
                            point.component(),
                            i.assignments,
                            None,
                            point.thread(),
                        ))
                    }

                    Control::Empty(_) => None,
                    // non-leaf nodes
                    Control::If(_)
                    | Control::While(_)
                    | Control::Repeat(_)
                    | Control::Seq(_)
                    | Control::Par(_) => None,
                }
            })
            .chain(self.env.pc.continuous_assigns().iter().map(|x| {
                ScheduledAssignments::new_continuous(x.comp, x.assigns)
            }))
            .chain(self.env.pc.with_map().iter().map(
                |(ctrl_pt, with_entry)| {
                    ScheduledAssignments::new_combinational(
                        ctrl_pt.comp,
                        self.ctx().primary[with_entry.group].assignments,
                    )
                },
            ))
            .collect();

        while let Some((cell, grp)) = additional_groups.pop_front() {
            if skiplist.insert((cell, grp)) {
                let group = &self.ctx().primary[grp];

                additional_groups.extend(
                    group
                        .structural_enables
                        .iter()
                        .copied()
                        .map(|grp| (cell, grp)),
                );

                out.push(ScheduledAssignments::new_control(
                    cell,
                    group.assignments,
                    Some(GroupInterfacePorts {
                        go: group.go,
                        done: group.done,
                    }),
                    None,
                ))
            }
        }

        out
    }

    /// A helper function which inserts indices for the ref cells and ports
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
            let done_port = self.env.unwrap_comp_done(*comp);
            let v = PortValue::new_implicit(BitVecValue::new_true());
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

        for point in vecs.iter() {
            let comp_done = self.env.unwrap_comp_done(point.component());
            let comp_go = self.env.unwrap_comp_go(point.component());
            let thread = point.thread().or_else(|| {
                self.env.ports[comp_go].as_option().and_then(|t| t.thread())
            });

            // if the done is not high & defined, we need to set it to low
            if !self.env.ports[comp_done].as_bool().unwrap_or_default() {
                self.env.ports[comp_done] =
                    PortValue::new_implicit(BitVecValue::new_false());
            }

            match &ctx_ref.primary[point.control_idx()].control {
                // actual nodes
                Control::Enable(enable) => {
                    let go_local = ctx_ref.primary[enable.group()].go;
                    let index_bases = &self.env.cells[point.component()]
                        .as_comp()
                        .unwrap()
                        .index_bases;

                    // set go high
                    let go_idx = index_bases + go_local;
                    self.env.ports[go_idx] =
                        PortValue::new_implicit(BitVecValue::new_true());
                }
                Control::Invoke(invoke) => {
                    if invoke.comb_group.is_some()
                        && !with_map.contains_key(point.control_point())
                    {
                        with_map.insert(
                            point.control_point().clone(),
                            WithEntry::new(invoke.comb_group.unwrap()),
                        );
                    }

                    let go =
                        self.get_global_port_idx(&invoke.go, point.component());
                    self.env.ports[go] =
                        PortValue::new_implicit(BitVecValue::new_true())
                            .with_thread_optional(
                                if self.conf.check_data_race {
                                    assert!(thread.is_some(), "Invoke is running but has no thread. This shouldn't happen. In {}", point.component().get_full_name(&self.env));
                                    thread
                                } else {
                                    None
                                },
                            );

                    // TODO griffin: should make this skip initialization if
                    // it's already initialized
                    self.initialize_ref_cells(point.component(), invoke);
                }
                // with nodes
                Control::If(i) => {
                    if i.cond_group().is_some()
                        && !with_map.contains_key(point.control_point())
                    {
                        with_map.insert(
                            point.control_point().clone(),
                            WithEntry::new(i.cond_group().unwrap()),
                        );
                    }
                }
                Control::While(w) => {
                    if w.cond_group().is_some()
                        && !with_map.contains_key(point.control_point())
                    {
                        with_map.insert(
                            point.control_point().clone(),
                            WithEntry::new(w.cond_group().unwrap()),
                        );
                    }
                }
                // --
                Control::Empty(_)
                | Control::Seq(_)
                | Control::Par(_)
                | Control::Repeat(_) => {}
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

        if self.conf.check_data_race {
            self.check_transitive_reads()?;
        }

        let mut changed = UpdateStatus::Unchanged;
        let mut prim_step_res = Ok(UpdateStatus::Unchanged);
        for cell in self.env.cells.values_mut() {
            match cell {
                CellLedger::Primitive { cell_dyn } => {
                    let res = cell_dyn.exec_cycle(
                        &mut self.env.ports,
                        &mut self.env.state_map,
                    );
                    match res {
                        Ok(c) => changed |= c,
                        Err(_) => {
                            prim_step_res = res;
                            break;
                        }
                    }
                }

                CellLedger::RaceDetectionPrimitive { cell_dyn } => {
                    let res = cell_dyn.exec_cycle_checked(
                        &mut self.env.ports,
                        &mut self.env.clocks,
                        &self.env.thread_map,
                        &mut self.env.state_map,
                    );
                    match res {
                        Ok(c) => changed |= c,
                        Err(_) => {
                            prim_step_res = res;
                            break;
                        }
                    }
                }
                CellLedger::Component(_) => {}
            }
        }
        if let Err(e) = prim_step_res {
            return Err(e.prettify_message(&self.env).into());
        }

        self.env.pc.clear_finished_comps();

        let mut new_nodes = vec![];
        let (mut vecs, mut par_map, mut with_map, mut repeat_map) =
            self.env.pc.take_fields();

        let mut removed = vec![];

        let mut i = 0;

        while i < vecs.len() {
            let node = &mut vecs[i];
            let (keep_node, node_changed) = self
                .evaluate_control_node(
                    node,
                    &mut new_nodes,
                    (&mut par_map, &mut with_map, &mut repeat_map),
                )
                .map_err(|e| e.prettify_message(&self.env))?;
            changed |= node_changed;
            match keep_node {
                ControlNodeEval::Reprocess => {
                    continue;
                }
                ControlNodeEval::Stop { retain_node } => {
                    if !retain_node {
                        removed.push(i);
                    }
                    i += 1;
                }
            }
        }
        let removed_empty = removed.is_empty();

        // should consider if swap remove is the right choice hre since it
        // breaks the list ordering which we then have to fix. Possibly a
        // standard remove would make more sense here? Or maybe an approach with
        // tombstones
        for i in removed.into_iter().rev() {
            vecs.swap_remove(i);
        }

        self.env
            .pc
            .restore_fields((vecs, par_map, with_map, repeat_map));

        let new_nodes_empty = new_nodes.is_empty();
        if !new_nodes_empty {
            self.policy
                .decide_new_nodes(&self.env.pc, &mut new_nodes)
                .map_err(|e| e.prettify_message(&self.env))?;
        }

        // insert all the new nodes from the par into the program counter
        self.env.pc.vec_mut().extend(new_nodes);

        // For thread propagation during race detection we need to iterate in
        // containment order. This probably isn't necessary for normal execution
        // and could be guarded by the `check_data_race` flag.
        //
        // If we altered the node list, we need to restore the order invariant
        if !removed_empty || !new_nodes_empty {
            changed = UpdateStatus::Changed;
            self.env.pc.vec_mut().sort_by_key(|x| x.component());
        }

        // Execution has stalled so we run the appropriate policy action
        if !changed.as_bool() && !self.is_done() {
            self.policy
                .decide_unpause(&mut self.env.pc)
                .map_err(|e| e.prettify_message(&self.env))?;
        }

        Ok(())
    }

    /// Visit each cell and for all non-combinational cells, check whether there
    /// are any reads that should be performed on their inputs that were
    /// deferred through combinational logic
    fn check_transitive_reads(&mut self) -> Result<(), BoxedCiderError> {
        let mut clock_map = std::mem::take(&mut self.env.clocks);
        for cell in self.env.cells.values() {
            if let Some(dyn_prim) = cell.as_primitive()
                && !dyn_prim.is_combinational() {
                    let sig = dyn_prim.get_ports();
                    for port in sig.iter_first() {
                        if let Some(val) = self.env.ports[port].as_option()
                            && val.propagate_clocks()
                                && (val.transitive_clocks().is_some())
                            {
                                // For non-combinational cells with
                                // transitive reads, we will check them at
                                // the cycle boundary and attribute the read
                                // to the continuous thread
                                let (assign_idx, cell) =
                                    val.winner().as_assign().unwrap();
                                self.check_read(
                                    ThreadMap::continuous_thread(),
                                    port,
                                    &mut clock_map,
                                    ReadSource::Assignment(assign_idx),
                                    cell,
                                )
                                .map_err(|e| e.prettify_message(&self.env))?
                            }
                    }
                }
        }
        self.env.clocks = clock_map;

        Ok(())
    }

    fn evaluate_control_node(
        &mut self,
        node: &mut ProgramPointer,
        new_nodes: &mut Vec<ProgramPointer>,
        maps: PcMaps,
    ) -> RuntimeResult<(ControlNodeEval, UpdateStatus)> {
        let (par_map, with_map, repeat_map) = maps;
        let comp_go = self.env.unwrap_comp_go(node.component());
        let comp_done = self.env.unwrap_comp_done(node.component());

        let thread = node.thread().or_else(|| {
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
            return Ok((ControlNodeEval::stop(true), UpdateStatus::Unchanged));
        }

        // This is a silly hack used to assess whether or not we updated the
        // node. I doubt it will become performance relevant, but a better
        // solution would probably be to have each function pass an UpdateStatus
        // value
        let node_orig = node.clone();

        let retain_bool = match &ctx.primary[node.control_idx()].control {
            Control::Seq(seq) => self.handle_seq(seq, node.control_point_mut()),
            Control::Par(par) => {
                let (ctrl_point, node_thread) = node.get_mut();
                self.handle_par(
                    par_map,
                    ctrl_point,
                    thread,
                    node_thread,
                    par,
                    new_nodes,
                )
            }
            Control::If(i) => {
                self.handle_if(with_map, node.control_point_mut(), thread, i)?
            }
            Control::While(w) => self.handle_while(
                w,
                with_map,
                node.control_point_mut(),
                thread,
            )?,
            Control::Repeat(rep) => {
                self.handle_repeat(repeat_map, node.control_point_mut(), rep)
            }

            // ===== leaf nodes =====
            Control::Empty(_) => node
                .control_point_mut()
                .mutate_into_next(self.env.ctx.as_ref()),

            Control::Enable(e) => {
                self.handle_enable(e, node.control_point_mut())
            }
            Control::Invoke(i) => {
                self.handle_invoke(i, node.control_point_mut(), with_map)?
            }
        };

        if retain_bool
            && self.conf.allow_multistep
            && node.control_point().should_reprocess(ctx)
        {
            // If we are re-processing a node then it necessarily changed
            return Ok((ControlNodeEval::Reprocess, UpdateStatus::Changed));
        }

        if !retain_bool && ControlPoint::get_next(node.control_point(), self.env.ctx.as_ref()).is_none() &&
         // either we are not a par node, or we are the last par node
         (!matches!(&self.env.ctx.as_ref().primary[node.control_point().control_node_idx].control, Control::Par(_)) || !par_map.contains_key(node.control_point()))
        {
            if self.conf.check_data_race {
                assert!(
                    thread.is_some(),
                    "finished comps should have a thread"
                );
            }

            self.env
                .pc
                .set_finished_comp(node.control_point().comp, thread);
            let comp_ledger =
                self.env.cells[node.control_point().comp].unwrap_comp();
            let new_point = node.control_point().new_retain_comp(
                self.env.ctx.as_ref().primary[comp_ledger.comp_id]
                    .unwrap_standard()
                    .control
                    .unwrap(),
            );
            node.set_control_point(new_point);
            Ok((ControlNodeEval::stop(true), UpdateStatus::Changed))
        } else {
            Ok((
                ControlNodeEval::stop(retain_bool),
                (*node != node_orig).into(),
            ))
        }
    }

    fn handle_seq(&mut self, seq: &Seq, node: &mut ControlPoint) -> bool {
        if !seq.is_empty() {
            let next = seq.stms()[0];
            *node = node.new_retain_comp(next);
            true
        } else {
            node.mutate_into_next(self.env.ctx.as_ref())
        }
    }

    fn handle_repeat(
        &mut self,
        repeat_map: &mut HashMap<ControlPoint, u64>,
        node: &mut ControlPoint,
        rep: &Repeat,
    ) -> bool {
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

    fn handle_enable(&mut self, e: &Enable, node: &mut ControlPoint) -> bool {
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

    fn handle_invoke(
        &mut self,
        i: &Invoke,
        node: &mut ControlPoint,
        with_map: &mut HashMap<ControlPoint, WithEntry>,
    ) -> Result<bool, BoxedRuntimeError> {
        let done = self.get_global_port_idx(&i.done, node.comp);
        if i.comb_group.is_some() && !with_map.contains_key(node) {
            with_map
                .insert(node.clone(), WithEntry::new(i.comb_group.unwrap()));
        }
        Ok(if !self.env.ports[done].as_bool().unwrap_or_default() {
            true
        } else {
            self.cleanup_ref_cells(node.comp, i);

            if i.comb_group.is_some() {
                with_map.remove(node);
            }

            node.mutate_into_next(self.env.ctx.as_ref())
        })
    }

    fn handle_par(
        &mut self,
        par_map: &mut HashMap<ControlPoint, ParEntry>,
        node: &mut ControlPoint,
        thread: Option<ThreadIdx>,
        node_thread: &mut Option<ThreadIdx>,
        par: &Par,
        new_nodes: &mut Vec<ProgramPointer>,
    ) -> bool {
        if par_map.contains_key(node) {
            let par_entry = par_map.get_mut(node).unwrap();
            *par_entry.child_count_mut() -= 1;

            if self.conf.check_data_race {
                par_entry.add_finished_thread(
                    thread.expect("par nodes should have a thread"),
                );
            }

            if par_entry.child_count() == 0 {
                let par_entry = par_map.remove(node).unwrap();
                if self.conf.check_data_race {
                    debug_assert!(
                        par_entry
                            .iter_finished_threads()
                            .map(|thread| {
                                self.env.thread_map[thread].parent().unwrap()
                            })
                            .all_equal()
                    );
                    let parent =
                        self.env.thread_map[thread.unwrap()].parent().unwrap();
                    let parent_clock =
                        self.env.thread_map.unwrap_clock_id(parent);

                    for child_thread in par_entry.iter_finished_threads() {
                        let child_clock_idx =
                            self.env.thread_map.unwrap_clock_id(child_thread);

                        let (parent_clock, child_clock) = self
                            .env
                            .clocks
                            .split_mut_indices(parent_clock, child_clock_idx)
                            .unwrap();

                        parent_clock.sync(child_clock);
                    }

                    *node_thread = par_entry.original_thread();
                    self.env.clocks[parent_clock].increment(&parent);
                }
                node.mutate_into_next(self.env.ctx.as_ref())
            } else {
                false
            }
        } else {
            if par.is_empty() {
                return node.mutate_into_next(self.env.ctx.as_ref());
            }

            par_map.insert(
                node.clone(),
                ParEntry::new(par.stms().len().try_into().expect(
                    "More than (2^16 - 1 threads) in a par block. Are you sure this is a good idea?",
                ), *node_thread)
                ,
            );
            new_nodes.extend(par.stms().iter().map(|x| {
                let thread = if self.conf.check_data_race {
                    let thread =
                        thread.expect("par nodes should have a thread");

                    let new_thread_idx: ThreadIdx = if self.conf.disable_memo {
                        self.env
                            .thread_map
                            .spawn(thread, self.env.clocks.new_clock())
                    } else {
                        *(self
                            .env
                            .pc
                            .lookup_thread(node.comp, thread, *x)
                            .or_insert_with(|| {
                                let new_clock_idx = self.env.clocks.new_clock();

                                self.env.thread_map.spawn(thread, new_clock_idx)
                            }))
                    };

                    let new_clock_idx =
                        self.env.thread_map.unwrap_clock_id(new_thread_idx);

                    self.env.clocks[new_clock_idx] = self.env.clocks
                        [self.env.thread_map.unwrap_clock_id(thread)]
                    .clone();

                    assert_eq!(
                        self.env.thread_map[new_thread_idx].parent().unwrap(),
                        thread
                    );

                    self.env.clocks[new_clock_idx].increment(&new_thread_idx);

                    Some(new_thread_idx)
                } else {
                    None
                };

                ProgramPointer::new_active(thread, node.new_retain_comp(*x))
            }));

            if self.conf.check_data_race {
                let thread = thread.expect("par nodes should have a thread");
                let clock = self.env.thread_map.unwrap_clock_id(thread);
                self.env.clocks[clock].increment(&thread);
            }

            false
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
            let mut clock_map = std::mem::take(&mut self.env.clocks);

            self.check_read_relative(
                thread.unwrap(),
                w.cond_port(),
                node.comp,
                &mut clock_map,
                ReadSource::Conditional(node.control_node_idx),
            )?;

            self.env.clocks = clock_map;
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
                let mut clock_map = std::mem::take(&mut self.env.clocks);

                self.check_read(
                    thread.unwrap(),
                    idx,
                    &mut clock_map,
                    ReadSource::Conditional(node.control_node_idx),
                    node.comp,
                )?;

                self.env.clocks = clock_map;
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
                    PortComp::Eq => a_val == b_val,
                    PortComp::Neq => a_val != b_val,
                    PortComp::Gt => a_val.is_greater(b_val),
                    PortComp::Lt => a_val.is_less(b_val),
                    PortComp::Geq => a_val.is_greater_or_equal(b_val),
                    PortComp::Leq => a_val.is_less_or_equal(b_val),
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
        for p in self.env.ports.values_mut() {
            *p = PortValue::new_undef()
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

        let mut changed_cells: FxHashSet<GlobalCellIdx> = FxHashSet::new();

        self.run_primitive_comb_path(self.env.cells.range().into_iter())?;
        let mut rerun_all_primitives = false;

        while has_changed {
            has_changed = false;

            // evaluate all the assignments and make updates
            for ScheduledAssignments {
                active_cell,
                assignments,
                interface_ports,
                thread,
                assign_type,
            } in assigns_bundle.iter()
            {
                let ledger = self.env.cells[*active_cell].as_comp().unwrap();
                let group_go = interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.go);
                let done = interface_ports
                    .as_ref()
                    .map(|x| &ledger.index_bases + x.done);

                let comp_go = self.env.get_comp_go(*active_cell);
                let thread = self.compute_thread(comp_go, thread, group_go);

                if self.is_assign_bundle_active(*assign_type, group_go, comp_go)
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

                            let dest = self
                                .get_global_port_idx(&assign.dst, *active_cell);

                            if let Some(done) = done
                                && dest != done {
                                    let done_val = &self.env.ports[done];

                                    if done_val.as_bool().unwrap_or(true) {
                                        // skip this assignment when we are done or
                                        // or the done signal is undefined
                                        continue;
                                    }
                                }

                            if self.conf.debug_logging {
                                self.log_assignment(
                                    active_cell,
                                    ledger,
                                    assign_idx,
                                    val,
                                );
                            }

                            if let Some(v) = val.as_option() {
                                let mut assigned_value = AssignedValue::new(
                                    v.val().clone(),
                                    (assign_idx, *active_cell),
                                );

                                // if this assignment is in a combinational
                                // context we want to propagate any clocks which
                                // are present. Since clocks aren't present
                                // when not running with `check_data_race`, this
                                // won't happen when the flag is not set
                                if self.conf.check_data_race
                                    && (val.clocks().is_some()
                                        || val.transitive_clocks().is_some())
                                    && (assign_type.is_combinational()
                                        || assign_type.is_continuous())
                                {
                                    assigned_value = assigned_value
                                        .with_transitive_clocks_opt(
                                            val.transitive_clocks().cloned(),
                                        )
                                        .with_propagate_clocks();
                                    // direct clock becomes a transitive clock
                                    // on assignment
                                    if let Some(c) = val.clocks() {
                                        assigned_value.add_transitive_clock(c);
                                    }
                                }

                                let result = self.env.ports.insert_val(
                                    dest,
                                    assigned_value.with_thread_optional(thread),
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

                                if changed.as_bool() && !rerun_all_primitives {
                                    changed_cells.insert(
                                        self.env.ports_to_cells_map[dest],
                                    );
                                }

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
                                todo!(
                                    "Raise an error here since this assignment is undefining things: {}. Port currently has value: {}",
                                    self.env
                                        .ctx
                                        .as_ref()
                                        .printer()
                                        .print_assignment(
                                            ledger.comp_id,
                                            assign_idx
                                        ),
                                    &self.env.ports[dest]
                                )
                            }
                        }
                    }
                }
            }

            if self.conf.check_data_race {
                self.propagate_comb_reads(assigns_bundle)?;
            }

            let changed = if rerun_all_primitives {
                rerun_all_primitives = false;
                self.run_primitive_comb_path(
                    self.env.cells.range().into_iter(),
                )?
            } else {
                self.run_primitive_comb_path(changed_cells.drain())?
            };

            has_changed |= changed;

            if !has_changed {
                for ScheduledAssignments {
                    active_cell,
                    interface_ports,
                    ..
                } in assigns_bundle.iter()
                {
                    if let Some(i) = interface_ports {
                        let ledger =
                            self.env.cells[*active_cell].as_comp().unwrap();
                        let go = &ledger.index_bases + i.go;
                        let done = &ledger.index_bases + i.done;

                        if self.env.ports[go].as_bool().unwrap_or_default()
                            && self.env.ports[done].is_undef()
                        {
                            self.env.ports[done] = PortValue::new_implicit(
                                BitVecValue::new_false(),
                            );
                            has_changed = true;
                        }
                    }
                }
            }

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
                        rerun_all_primitives = true;

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

        // check reads needs to happen before zeroing the go ports. If this is
        // not observed then some read checks will be accidentally skipped
        if self.conf.check_data_race {
            self.handle_reads(assigns_bundle)?;
        }

        if self.conf.undef_guard_check {
            self.check_undefined_guards(assigns_bundle)?;
        }

        // This should be the last update that occurs during convergence
        self.zero_done_groups_go(assigns_bundle);

        if self.conf.debug_logging {
            info!(self.env.logger, "Finished combinational convergence");
        }

        Ok(())
    }

    fn is_assign_bundle_active(
        &self,
        assign_type: AssignType,
        group_go: Option<GlobalPortIdx>,
        comp_go: Option<GlobalPortIdx>,
    ) -> bool {
        group_go
            .as_ref()
            // the group must have its go signal high and the go
            // signal of the component must also be high
            .map(|g| {
                self.env.ports[*g].as_bool().unwrap_or_default()
                    && self.env.ports[comp_go.unwrap()]
                        .as_bool()
                        .unwrap_or_default()
            })
            .unwrap_or_else(|| {
                // if there is no go signal, then we want to run the
                // continuous assignments but not comb group assignments
                if assign_type.is_continuous() {
                    true
                } else {
                    self.env.ports[comp_go.unwrap()]
                        .as_bool()
                        .unwrap_or_default()
                }
            })
    }

    /// For all groups in the given assignments, set the go port to zero if the
    /// done port is high
    fn zero_done_groups_go(&mut self, assigns_bundle: &[ScheduledAssignments]) {
        for ScheduledAssignments {
            active_cell,
            interface_ports,
            ..
        } in assigns_bundle.iter()
        {
            if let Some(interface_ports) = interface_ports {
                let ledger = self.env.cells[*active_cell].as_comp().unwrap();
                let go = &ledger.index_bases + interface_ports.go;
                let done = &ledger.index_bases + interface_ports.done;
                if self.env.ports[done].as_bool().unwrap_or_default() {
                    self.env.ports[go] =
                        PortValue::new_implicit(BitVecValue::zero(1));
                }
            }
        }
    }

    /// A final pass meant to be run after convergence which does the following:
    ///
    /// 1. For successful assignments, check reads from the source port if applicable
    /// 2. For non-continuous/combinational contexts, check all reads performed
    ///    by the guard regardless of whether the assignment fired or not
    /// 3. For continuous/combinational contexts, update the transitive reads of
    ///    the value in the destination with the reads done by the guard,
    ///    regardless of success
    fn handle_reads(
        &mut self,
        assigns_bundle: &[ScheduledAssignments],
    ) -> Result<(), BoxedRuntimeError> {
        // needed for mutability reasons
        let mut clock_map = std::mem::take(&mut self.env.clocks);

        for ScheduledAssignments {
            active_cell,
            assignments,
            interface_ports,
            thread,
            assign_type,
        } in assigns_bundle.iter()
        {
            let ledger = self.env.cells[*active_cell].as_comp().unwrap();
            let go =
                interface_ports.as_ref().map(|x| &ledger.index_bases + x.go);

            let comp_go = self.env.get_comp_go(*active_cell);

            let thread = self.compute_thread(comp_go, thread, go);

            // check for direct reads
            if assign_type.is_control()
                && go
                    .as_ref()
                    .map(|g| {
                        self.env.ports[*g].as_bool().unwrap_or_default()
                            && self.env.ports[comp_go.unwrap()]
                                .as_bool()
                                .unwrap_or_default()
                    })
                    .unwrap_or_default()
            {
                for assign_idx in assignments.iter() {
                    let assign = &self.env.ctx.as_ref().primary[assign_idx];

                    // read source
                    if self
                        .evaluate_guard(assign.guard, *active_cell)
                        .unwrap_or_default()
                    {
                        self.check_read_relative(
                            thread.unwrap(),
                            assign.src,
                            *active_cell,
                            &mut clock_map,
                            ReadSource::Assignment(assign_idx),
                        )?;
                    }

                    // guard reads, assignment firing does not matter
                    if let Some(read_ports) = self
                        .env
                        .ctx
                        .as_ref()
                        .primary
                        .guard_read_map
                        .get(assign.guard)
                    {
                        for port in read_ports {
                            self.check_read_relative(
                                thread.unwrap(),
                                *port,
                                *active_cell,
                                &mut clock_map,
                                ReadSource::Guard(assign_idx),
                            )?;
                        }
                    }
                }
            }
        }
        self.env.clocks = clock_map;

        Ok(())
    }

    /// For continuous/combinational contexts, update the transitive reads of
    ///     the value in the destination with the reads done by the guard,
    ///     regardless of success
    fn propagate_comb_reads(
        &mut self,
        assigns_bundle: &[ScheduledAssignments],
    ) -> Result<(), BoxedRuntimeError> {
        let mut set_extension = HashSet::new();

        for ScheduledAssignments {
            active_cell,
            assignments,
            assign_type,
            ..
        } in assigns_bundle.iter()
        {
            let comp_go = self.env.get_comp_go(*active_cell);

            if (assign_type.is_combinational()
                && comp_go
                    .and_then(|comp_go| self.env.ports[comp_go].as_bool())
                    .unwrap_or_default())
                || assign_type.is_continuous()
            {
                for assign_idx in assignments.iter() {
                    let assign = &self.env.ctx.as_ref().primary[assign_idx];
                    let dest =
                        self.get_global_port_idx(&assign.dst, *active_cell);

                    if let Some(read_ports) = self
                        .env
                        .ctx
                        .as_ref()
                        .primary
                        .guard_read_map
                        .get(assign.guard)
                        && self.env.ports[dest].is_def() {
                            for port in read_ports {
                                let port = self
                                    .get_global_port_idx(port, *active_cell);
                                if let Some(clock) =
                                    self.env.ports[port].clocks()
                                {
                                    set_extension.insert(clock);
                                }
                                if let Some(clocks) =
                                    self.env.ports[port].transitive_clocks()
                                {
                                    set_extension
                                        .extend(clocks.iter().copied());
                                }
                            }
                            if !set_extension.is_empty() {
                                self.env.ports[dest]
                                    .as_option_mut()
                                    .unwrap()
                                    .add_transitive_clocks(
                                        set_extension.drain(),
                                    );
                            }

                            // this is necessary for ports which were implicitly
                            // assigned zero and is redundant for other ports
                            // which will already have propagate_clocks set
                            self.env.ports[dest]
                                .as_option_mut()
                                .unwrap()
                                .set_propagate_clocks(true);
                        }
                }
            }
        }

        Ok(())
    }

    /// Check for undefined guards and raise an error if any are found
    fn check_undefined_guards(
        &mut self,
        assigns_bundle: &[ScheduledAssignments],
    ) -> Result<(), BoxedRuntimeError> {
        let mut error_v = vec![];
        for bundle in assigns_bundle.iter() {
            let ledger = self.env.cells[bundle.active_cell].as_comp().unwrap();
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
                                let p = self
                                    .get_global_port_idx(p, bundle.active_cell);
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
            Err(RuntimeError::UndefinedGuardError(error_v).into())
        } else {
            Ok(())
        }
    }

    fn run_primitive_comb_path<I>(
        &mut self,
        cells_to_run: I,
    ) -> Result<bool, BoxedRuntimeError>
    where
        I: Iterator<Item = GlobalCellIdx>,
    {
        if self.conf.debug_logging {
            info!(self.env().logger, "Starting primitive combinational update");
        }

        let mut changed = UpdateStatus::Unchanged;

        let mut working_set = vec![];
        for cell in cells_to_run {
            let cell = &mut self.env.cells[cell];
            match cell {
                CellLedger::Primitive { cell_dyn } => {
                    let result = cell_dyn
                        .exec_comb(&mut self.env.ports, &self.env.state_map)?;

                    changed |= result;

                    if self.conf.check_data_race && cell_dyn.is_combinational()
                    {
                        let signature = cell_dyn.get_ports();

                        for port in signature.iter_first() {
                            let val = &self.env.ports[port];
                            if let Some(val) = val.as_option()
                                && val.propagate_clocks()
                                    && (val.clocks().is_some()
                                        || val.transitive_clocks().is_some())
                                {
                                    if let Some(clocks) = val.clocks() {
                                        working_set.push(*clocks);
                                    }
                                    working_set
                                        .extend(val.iter_transitive_clocks());
                                }
                        }

                        if signature.iter_second().len() == 1 {
                            let port = signature.iter_second().next().unwrap();
                            let val = &mut self.env.ports[port];
                            if let Some(val) = val.as_option_mut()
                                && !working_set.is_empty()
                            {
                                val.add_transitive_clocks(
                                    working_set.drain(..),
                                );
                            }
                        } else {
                            todo!("comb primitive with multiple outputs")
                        }
                    }
                }
                CellLedger::RaceDetectionPrimitive { cell_dyn } => {
                    let result = cell_dyn.exec_comb_checked(
                        &mut self.env.ports,
                        &mut self.env.clocks,
                        &self.env.thread_map,
                        &self.env.state_map,
                    )?;
                    changed |= result;
                }

                CellLedger::Component(_) => {}
            }
        }

        if self.conf.debug_logging {
            info!(self.env().logger, "Finished primitive combinational update");
        }

        Ok(changed.as_bool())
    }

    /// A wrapper function for [check_read] which takes in a [PortRef] and the
    /// active component cell and calls [check_read] with the appropriate [GlobalPortIdx]
    fn check_read_relative(
        &self,
        thread: ThreadIdx,
        port: PortRef,
        active_cell: GlobalCellIdx,
        clock_map: &mut ClockMap,
        source: ReadSource,
    ) -> Result<(), BoxedRuntimeError> {
        let global_port = self.get_global_port_idx(&port, active_cell);
        self.check_read(thread, global_port, clock_map, source, active_cell)
    }

    fn check_read(
        &self,
        thread: ThreadIdx,
        global_port: GlobalPortIdx,
        clock_map: &mut ClockMap,
        source: ReadSource,
        cell: GlobalCellIdx,
    ) -> Result<(), BoxedRuntimeError> {
        let val = &self.env.ports[global_port];
        let thread_clock = self.env.thread_map.unwrap_clock_id(thread);

        if val.clocks().is_some() && val.transitive_clocks().is_some() {
            // TODO griffin: Sort this out
            panic!(
                "Value has both direct clock and transitive clock. This shouldn't happen?"
            )
        } else if let Some(clocks) = val.clocks() {
            clocks
                .check_read_with_ascription(
                    (thread, thread_clock),
                    source,
                    cell,
                    clock_map,
                )
                .map_err(|e| {
                    let info = clock_map.lookup_cell(clocks).expect("Clock pair without cell. This should never happen, please report this bug");
                    e.add_cell_info(info.attached_cell, info.entry_number)
                })?
        } else if let Some(transitive_clocks) = val.transitive_clocks() {
            for clock_pair in transitive_clocks.iter() {
                clock_pair
                    .check_read_with_ascription(
                        (thread, thread_clock),
                        source.clone(),
                        cell,
                        clock_map,
                    )
                    .map_err(|e| {
                        let info = clock_map.lookup_cell(*clock_pair).expect("Clock pair without cell. This should never happen, please report this bug");
                        e.add_cell_info(info.attached_cell, info.entry_number)
                    })?
            }
        }

        Ok(())
    }

    fn log_assignment(
        &self,
        active_cell: &GlobalCellIdx,
        ledger: &ComponentLedger,
        assign_idx: AssignmentIdx,
        val: &PortValue,
    ) {
        info!(
            self.env.logger,
            "Assignment fired in {}: {}\n     wrote {}",
            self.env.get_full_name(active_cell),
            self.ctx()
                .printer()
                .print_assignment(ledger.comp_id, assign_idx)
                .yellow(),
            val.bold()
        );
    }

    /// Attempts to compute the thread id for the given group/component.
    ///
    /// If the given thread is `None`, then the thread id is computed from the
    /// go port for the group. If no such port exists, or it lacks a thread id,
    /// then the thread id is computed from the go port for the component. If
    /// none of these succeed then `None` is returned.
    fn compute_thread(
        &self,
        comp_go: Option<GlobalPortIdx>,
        thread: &Option<ThreadIdx>,
        go: Option<GlobalPortIdx>,
    ) -> Option<ThreadIdx> {
        thread.or_else(|| {
            if let Some(go_idx) = go
                && let Some(go_thread) =
                    self.env.ports[go_idx].as_option().and_then(|a| a.thread())
                {
                    return Some(go_thread);
                }
            comp_go.and_then(|comp_go| {
                self.env.ports[comp_go].as_option().and_then(|x| x.thread())
            })
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
                CellPrototype::Memory(MemoryPrototype {
                    width,
                    dims,
                    is_external,
                    ..
                }) if *is_external | all_mems => {
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
                            .serializer()
                            .unwrap()
                            .dump_data(&self.env.state_map),
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
                                .serializer()
                                .unwrap()
                                .dump_data(&self.env.state_map),
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
        name: Option<impl AsRef<str>>,
    ) -> String {
        let mut buf = String::new();

        if let Some(name_override) = name {
            writeln!(buf, "{}:", name_override.as_ref().stylize_name())
                .unwrap();
        } else {
            writeln!(buf, "{}:", self.get_full_name(cell_idx).stylize_name())
                .unwrap();
        }
        for (identifier, port_idx) in self.env.get_ports_from_cell(cell_idx) {
            writeln!(
                buf,
                "  {}: {}",
                self.ctx().lookup_name(identifier).stylize_port_name(),
                self.format_port_value(port_idx, print_code).stylize_value()
            )
            .unwrap();
        }

        buf
    }

    /// return a string containing formatted data for the given target, assuming
    /// there is internal state to inspect and that the target does not contain
    /// an invalid address
    pub fn format_cell_state(
        &self,
        cell_idx: GlobalCellIdx,
        print_code: PrintCode,
        target: &PrintTarget,
    ) -> Option<String> {
        let cell = self.env.cells[cell_idx].unwrap_primitive();
        let state = cell
            .serializer()?
            .serialize(print_code, &self.env.state_map);

        if let Some(addr) = target.address() {
            state.format_address(addr)
        } else {
            Some(format!("{state}"))
        }
    }
}

/// The standard simulator used by Cider with the option to write out a waveform.
pub struct Simulator<C: AsRef<Context> + Clone> {
    base: BaseSimulator<C>,
    wave: Option<WaveWriter>,
}

impl<C: AsRef<Context> + Clone> std::ops::Deref for Simulator<C> {
    type Target = BaseSimulator<C>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<C: AsRef<Context> + Clone> std::ops::DerefMut for Simulator<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<C: AsRef<Context> + Clone> Simulator<C> {
    pub fn build_simulator(
        ctx: C,
        data_file: &Option<std::path::PathBuf>,
        wave_file: &Option<std::path::PathBuf>,
        runtime_config: RuntimeConfig,
        policy_choice: PolicyChoice,
    ) -> Result<Self, BoxedCiderError> {
        let data_dump = if let Some(path) = data_file {
            let mut file = std::fs::File::open(path)?;
            Some(DataDump::deserialize(&mut file)?)
        } else {
            None
        };
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
            base: BaseSimulator::new(
                env,
                runtime_config,
                policy_choice.generate_policy(),
            ),
            wave,
        })
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
                        e.stylize_error()
                    );
                }
                Err(e)
            }
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
