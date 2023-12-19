use ahash::{HashSet, HashSetExt};
use itertools::Itertools;

use super::{assignments::AssignmentBundle, program_counter::ProgramCounter};

use super::super::{
    context::Context, index_trait::IndexRange, indexed_map::IndexedMap,
};
use crate::{
    errors::InterpreterResult,
    flatten::{
        flat_ir::{
            prelude::{
                AssignmentIdx, BaseIndices, ComponentIdx, ControlIdx,
                ControlMap, ControlNode, GlobalCellIdx, GlobalPortIdx,
                GlobalPortRef, GlobalRefCellIdx, GlobalRefPortIdx, GuardIdx,
                PortRef,
            },
            wires::guards::Guard,
        },
        primitives::{
            self,
            prim_trait::{AssignResult, UpdateStatus},
            Primitive,
        },
        structures::{
            environment::program_counter::{ControlPoint, SearchPath},
            index_trait::IndexRef,
        },
    },
    values::Value,
};
use std::{collections::VecDeque, fmt::Debug};

pub type PortMap = IndexedMap<GlobalPortIdx, Value>;

impl PortMap {
    pub fn insert_val(
        &mut self,
        target: GlobalPortIdx,
        val: Value,
    ) -> UpdateStatus {
        if self[target] != val {
            self[target] = val;
            UpdateStatus::Changed
        } else {
            UpdateStatus::Unchanged
        }
    }
}

pub(crate) type CellMap = IndexedMap<GlobalCellIdx, CellLedger>;
pub(crate) type RefCellMap =
    IndexedMap<GlobalRefCellIdx, Option<GlobalCellIdx>>;
pub(crate) type RefPortMap =
    IndexedMap<GlobalRefPortIdx, Option<GlobalPortIdx>>;
pub(crate) type AssignmentRange = IndexRange<AssignmentIdx>;

pub(crate) struct ComponentLedger {
    pub(crate) index_bases: BaseIndices,
    pub(crate) comp_id: ComponentIdx,
}

impl ComponentLedger {
    /// Convert a relative offset to a global one. Perhaps should take an owned
    /// value rather than a pointer
    pub fn convert_to_global(&self, port: &PortRef) -> GlobalPortRef {
        match port {
            PortRef::Local(l) => (&self.index_bases + l).into(),
            PortRef::Ref(r) => (&self.index_bases + r).into(),
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
    Component(ComponentLedger),
}

impl CellLedger {
    fn new_comp(idx: ComponentIdx, env: &Environment) -> Self {
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
}

impl Debug for CellLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive { .. } => f.debug_struct("Primitive").finish(),
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

#[derive(Debug)]
pub struct Environment<'a> {
    /// A map from global port IDs to their current values.
    pub(crate) ports: PortMap,
    /// A map from global cell IDs to their current state and execution info.
    cells: CellMap,
    /// A map from global ref cell IDs to the cell they reference, if any.
    ref_cells: RefCellMap,
    /// A map from global ref port IDs to the port they reference, if any.
    ref_ports: RefPortMap,

    /// The program counter for the whole program execution.
    pc: ProgramCounter,

    /// The immutable context. This is retained for ease of use.
    ctx: &'a Context,
}

impl<'a> Environment<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        let root = ctx.entry_point;
        let aux = &ctx.secondary[root];

        let mut env = Self {
            ports: PortMap::with_capacity(aux.port_offset_map.count()),
            cells: CellMap::with_capacity(aux.cell_offset_map.count()),
            ref_cells: RefCellMap::with_capacity(
                aux.ref_cell_offset_map.count(),
            ),
            ref_ports: RefPortMap::with_capacity(
                aux.ref_port_offset_map.count(),
            ),
            pc: ProgramCounter::new(ctx),
            ctx,
        };

        let root_node = CellLedger::new_comp(root, &env);
        let root = env.cells.push(root_node);
        env.layout_component(root);

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
    fn layout_component(&mut self, comp: GlobalCellIdx) {
        let ComponentLedger {
            index_bases,
            comp_id,
        } = self.cells[comp]
            .as_comp()
            .expect("Called layout component with a non-component cell.");
        let comp_aux = &self.ctx.secondary[*comp_id];

        let comp_id = *comp_id;

        // first layout the signature
        for sig_port in comp_aux.signature.iter() {
            let width = self.ctx.lookup_port_def(&comp_id, sig_port).width;
            let idx = self.ports.push(Value::zeroes(width));
            debug_assert_eq!(index_bases + sig_port, idx);
        }
        // second group ports
        for group_idx in comp_aux.definitions.groups() {
            //go
            let go = self.ports.push(Value::bit_low());

            //done
            let done = self.ports.push(Value::bit_low());

            // quick sanity check asserts
            let go_actual = index_bases + self.ctx.primary[group_idx].go;
            let done_actual = index_bases + self.ctx.primary[group_idx].done;
            // Case 1 - Go defined before done
            if self.ctx.primary[group_idx].go < self.ctx.primary[group_idx].done
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
        }

        for (cell_off, def_idx) in comp_aux.cell_offset_map.iter() {
            let info = &self.ctx.secondary[*def_idx];
            if !info.prototype.is_component() {
                let port_base = self.ports.peek_next_idx();
                for port in info.ports.iter() {
                    let width = self.ctx.lookup_port_def(&comp_id, port).width;
                    let idx = self.ports.push(Value::zeroes(width));
                    debug_assert_eq!(
                        &self.cells[comp].as_comp().unwrap().index_bases + port,
                        idx
                    );
                }
                let cell_dyn =
                    primitives::build_primitive(self, info, port_base);
                let cell = self.cells.push(CellLedger::Primitive { cell_dyn });

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

                self.layout_component(cell);
            }
        }

        // ref cells and ports are initialized to None
        for (ref_cell, def_idx) in comp_aux.ref_cell_offset_map.iter() {
            let info = &self.ctx.secondary[*def_idx];
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
}

// ===================== Environment print implementations =====================
impl<'a> Environment<'a> {
    pub fn print_env(&self) {
        let root_idx = GlobalCellIdx::new(0);
        let mut hierarchy = Vec::new();
        self.print_component(root_idx, &mut hierarchy)
    }

    fn print_component(
        &self,
        target: GlobalCellIdx,
        hierarchy: &mut Vec<GlobalCellIdx>,
    ) {
        let info = self.cells[target].as_comp().unwrap();
        let comp = &self.ctx.secondary[info.comp_id];
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
                let prior_comp = &self.ctx.secondary[info.comp_id];
                &self.ctx.secondary[prior_comp.name]
            })
            .chain(hierarchy.iter().zip(hierarchy.iter().skip(1)).map(
                |(l, r)| {
                    let info = self.cells[*l].as_comp().unwrap();
                    let prior_comp = &self.ctx.secondary[info.comp_id];
                    let local_target = r - (&info.index_bases);

                    let def_idx = &prior_comp.cell_offset_map[local_target];

                    let id = &self.ctx.secondary[*def_idx];
                    &self.ctx.secondary[id.name]
                },
            ))
            .join(".");

        for (cell_off, def_idx) in comp.cell_offset_map.iter() {
            let definition = &self.ctx.secondary[*def_idx];

            println!("{}.{}", name_prefix, self.ctx.secondary[definition.name]);
            for port in definition.ports.iter() {
                let definition =
                    &self.ctx.secondary[comp.port_offset_map[port]];
                println!(
                    "    {}: {}",
                    self.ctx.secondary[definition.name],
                    self.ports[&info.index_bases + port]
                );
            }

            if definition.prototype.is_component() {
                let child_target = &info.index_bases + cell_off;
                self.print_component(child_target, hierarchy);
            }
        }

        hierarchy.pop();
    }

    pub fn print_env_stats(&self) {
        println!("Environment Stats:");
        println!("  Ports: {}", self.ports.len());
        println!("  Cells: {}", self.cells.len());
        println!("  Ref Cells: {}", self.ref_cells.len());
        println!("  Ref Ports: {}", self.ref_ports.len());
    }

    pub fn print_pc(&self) {
        println!("{:?}", self.pc)
    }
}

/// A wrapper struct for the environment that provides the functions used to
/// simulate the actual program
pub struct Simulator<'a> {
    env: Environment<'a>,
}

impl<'a> Simulator<'a> {
    pub fn new(env: Environment<'a>) -> Self {
        Self { env }
    }

    pub fn print_env(&self) {
        self.env.print_env()
    }

    pub fn ctx(&self) -> &Context {
        self.env.ctx
    }
}

// =========================== simulation functions ===========================
impl<'a> Simulator<'a> {
    /// pull out the next nodes to search when
    fn extract_next_search(&self, idx: ControlIdx) -> VecDeque<ControlIdx> {
        match &self.env.ctx.primary[idx] {
            ControlNode::Seq(s) => s.stms().iter().copied().collect(),
            ControlNode::Par(p) => p.stms().iter().copied().collect(),
            ControlNode::If(i) => vec![i.tbranch(), i.fbranch()].into(),
            ControlNode::While(w) => vec![w.body()].into(),
            _ => VecDeque::new(),
        }
    }

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
    fn get_global_idx(
        &self,
        port: &PortRef,
        comp: GlobalCellIdx,
    ) -> GlobalPortIdx {
        let ledger = self.env.cells[comp].unwrap_comp();
        self.lookup_global_port_id(ledger.convert_to_global(port))
    }

    #[inline]
    fn get_value(&self, port: &PortRef, comp: GlobalCellIdx) -> &Value {
        let port_idx = self.get_global_idx(port, comp);
        &self.env.ports[port_idx]
    }

    fn get_parent_cell(
        &self,
        port: PortRef,
        comp: GlobalCellIdx,
    ) -> GlobalCellIdx {
        let component = self.env.cells[comp].unwrap_comp();
        let comp_info = &self.env.ctx.secondary[component.comp_id];

        match port {
            PortRef::Local(l) => {
                for (cell_offset, cell_def_idx) in
                    comp_info.cell_offset_map.iter()
                {
                    if self.env.ctx.secondary[*cell_def_idx].ports.contains(l) {
                        return &component.index_bases + cell_offset;
                    }
                }
            }
            PortRef::Ref(r) => {
                for (cell_offset, cell_def_idx) in
                    comp_info.ref_cell_offset_map.iter()
                {
                    if self.env.ctx.secondary[*cell_def_idx].ports.contains(r) {
                        let ref_cell_idx = &component.index_bases + cell_offset;
                        return self.env.ref_cells[ref_cell_idx]
                            .expect("Ref cell has not been instantiated");
                    }
                }
            }
        }

        unreachable!("Port does not exist on given component. This is an error please report it")
    }

    // may want to make this iterate directly if it turns out that the vec
    // allocation is too expensive in this context
    fn get_assignments(
        &self,
        control_points: &[ControlPoint],
    ) -> AssignmentBundle {
        control_points
            .iter()
            .map(|node| {
                match &self.ctx().primary[node.control_node] {
                    ControlNode::Enable(e) => {
                        (node.comp, self.ctx().primary[e.group()].assignments)
                    }

                    ControlNode::Invoke(_) => {
                        todo!("invokes not yet implemented")
                    }

                    ControlNode::Empty(_) => {
                        unreachable!(
                            "called `get_assignments` with an empty node"
                        )
                    }
                    // non-leaf nodes
                    ControlNode::If(_)
                    | ControlNode::While(_)
                    | ControlNode::Seq(_)
                    | ControlNode::Par(_) => {
                        unreachable!(
                            "Called `get_assignments` with non-leaf nodes"
                        )
                    }
                }
            })
            .collect()
    }

    pub fn step(&mut self) -> InterpreterResult<()> {
        /// attempts to get the next node for the given control point, if found
        /// it replaces the given node. Returns true if the node was found and
        /// replaced, returns false otherwise
        fn get_next(node: &mut ControlPoint, ctx: &Context) -> bool {
            let path = SearchPath::find_path_from_root(node.control_node, ctx);
            let next = path.next_node(&ctx.primary.control);
            if let Some(next) = next {
                *node = node.new_w_comp(next);
                true
            } else {
                //need to remove the node from the list now
                false
            }
        }

        // place to keep track of what groups we need to conclude at the end of
        // this step. These are indices into the program counter

        let mut leaf_nodes = vec![];

        self.env.pc.vec_mut().retain_mut(|node| {
            // just considering a single node case for the moment
            match &self.env.ctx.primary[node.control_node] {
                ControlNode::Seq(seq) => {
                    if !seq.is_empty() {
                        let next = seq.stms()[0];
                        *node = node.new_w_comp(next);
                        true
                    } else {
                        get_next(node, self.env.ctx)
                    }
                }
                ControlNode::Par(_par) => todo!("not ready for par yet"),
                ControlNode::If(i) => {
                    if i.cond_group().is_some() {
                        todo!("if statement has a with clause")
                    }

                    let target = GlobalPortRef::from_local(
                        i.cond_port(),
                        &self.env.cells[node.comp].unwrap_comp().index_bases,
                    );

                    let result = match target {
                        GlobalPortRef::Port(p) => self.env.ports[p].as_bool(),
                        GlobalPortRef::Ref(r) => {
                            let index = self.env.ref_ports[r].unwrap();
                            self.env.ports[index].as_bool()
                        }
                    };

                    let target = if result { i.tbranch() } else { i.fbranch() };
                    *node = node.new_w_comp(target);
                    true
                }
                ControlNode::While(w) => {
                    if w.cond_group().is_some() {
                        todo!("while statement has a with clause")
                    }

                    let target = GlobalPortRef::from_local(
                        w.cond_port(),
                        &self.env.cells[node.comp].unwrap_comp().index_bases,
                    );

                    let result = match target {
                        GlobalPortRef::Port(p) => self.env.ports[p].as_bool(),
                        GlobalPortRef::Ref(r) => {
                            let index = self.env.ref_ports[r].unwrap();
                            self.env.ports[index].as_bool()
                        }
                    };

                    if result {
                        // enter the body
                        *node = node.new_w_comp(w.body());
                        true
                    } else {
                        // ascend the tree
                        get_next(node, self.env.ctx)
                    }
                }

                // ===== leaf nodes =====
                ControlNode::Empty(_) => get_next(node, self.env.ctx),
                ControlNode::Enable(_) => {
                    leaf_nodes.push(node.clone());
                    true
                }
                ControlNode::Invoke(_) => todo!("invokes not implemented yet"),
            }
        });

        // we want to iterate through all the nodes present in the program counter

        // first we need to check for conditional control nodes

        // self.simulate_combinational();

        Ok(())
    }

    fn evaluate_guard(&self, guard: GuardIdx, comp: GlobalCellIdx) -> bool {
        let guard = &self.ctx().primary[guard];
        match guard {
            Guard::True => true,
            Guard::Or(a, b) => {
                self.evaluate_guard(*a, comp) || self.evaluate_guard(*b, comp)
            }
            Guard::And(a, b) => {
                self.evaluate_guard(*a, comp) || self.evaluate_guard(*b, comp)
            }
            Guard::Not(n) => !self.evaluate_guard(*n, comp),
            Guard::Comp(c, a, b) => {
                let comp_v = self.env.cells[comp].unwrap_comp();

                let a = self.lookup_global_port_id(comp_v.convert_to_global(a));
                let b = self.lookup_global_port_id(comp_v.convert_to_global(b));

                let a_val = &self.env.ports[a];
                let b_val = &self.env.ports[b];
                match c {
                    calyx_ir::PortComp::Eq => a_val == b_val,
                    calyx_ir::PortComp::Neq => a_val != b_val,
                    calyx_ir::PortComp::Gt => a_val > b_val,
                    calyx_ir::PortComp::Lt => a_val < b_val,
                    calyx_ir::PortComp::Geq => a_val >= b_val,
                    calyx_ir::PortComp::Leq => a_val <= b_val,
                }
            }
            Guard::Port(p) => {
                let comp_v = self.env.cells[comp].unwrap_comp();
                let p_idx =
                    self.lookup_global_port_id(comp_v.convert_to_global(p));
                self.env.ports[p_idx].as_bool()
            }
        }
    }

    fn simulate_combinational(
        &mut self,
        control_points: &[ControlPoint],
    ) -> InterpreterResult<()> {
        let assigns_bundle = self.get_assignments(control_points);
        let mut has_changed = true;

        let parent_cells: HashSet<GlobalCellIdx> = assigns_bundle
            .iter()
            .flat_map(|(cell, assigns)| {
                assigns.iter().map(|x| {
                    let assign = &self.env.ctx.primary[x];
                    self.get_parent_cell(assign.dst, *cell)
                })
            })
            .collect();

        while has_changed {
            has_changed = false;

            // evaluate all the assignments and make updates
            for (cell, assigns) in assigns_bundle.iter() {
                for assign in assigns {
                    let assign = &self.env.ctx.primary[assign];
                    if self.evaluate_guard(assign.guard, *cell) {
                        let val = self.get_value(&assign.src, *cell);
                        let dest = self.get_global_idx(&assign.dst, *cell);
                        if &self.env.ports[dest] != val {
                            has_changed = true;
                            self.env.ports[dest] = val.clone();
                        }
                    }
                }
            }

            // // This is incredibly silly
            // let results: InterpreterResult<Vec<AssignResult>> = parent_cells
            //     .iter()
            //     .map(|x| match &mut self.env.cells[*x] {
            //         CellLedger::Primitive { cell_dyn } => cell_dyn
            //             .exec_comb(&self.env.ports)
            //             .map(|x| x.into_iter()),
            //         CellLedger::Component(_) => todo!(),
            //     })
            //     .flatten_ok()
            //     .collect();

            // for AssignResult { destination, value } in results? {

            // }

            // run the primitives
        }

        Ok(())
    }

    pub fn _main_test(&mut self) {
        self.env.print_pc();
        for _x in self.env.pc.iter() {
            // println!("{:?} next {:?}", x, self.find_next_control_point(x))
        }
        self.step();
        self.step();
        self.env.print_pc();
        // println!("{:?}", self.get_assignments())
    }
}
