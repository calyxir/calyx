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
                ControlMap, ControlNode, GlobalCellId, GlobalPortId,
                GlobalPortRef, GlobalRefCellId, GlobalRefPortId, GuardIdx,
                PortRef,
            },
            wires::guards::Guard,
        },
        primitives::{self, Primitive},
        structures::{
            environment::program_counter::{ControlPoint, SearchPath},
            index_trait::IndexRef,
        },
    },
    values::Value,
};
use std::{collections::VecDeque, fmt::Debug};

pub type PortMap = IndexedMap<GlobalPortId, Value>;
pub(crate) type CellMap = IndexedMap<GlobalCellId, CellLedger>;
pub(crate) type RefCellMap = IndexedMap<GlobalRefCellId, Option<GlobalCellId>>;
pub(crate) type RefPortMap = IndexedMap<GlobalRefPortId, Option<GlobalPortId>>;
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
    fn layout_component(&mut self, comp: GlobalCellId) {
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
        let root_idx = GlobalCellId::new(0);
        let mut hierarchy = Vec::new();
        self.print_component(root_idx, &mut hierarchy)
    }

    fn print_component(
        &self,
        target: GlobalCellId,
        hierarchy: &mut Vec<GlobalCellId>,
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
    /// Finds the next control point from a finished control point. If there is
    /// no next control point, returns None.
    ///
    /// If given an If/While statement this will assume the entire if/while node
    /// is finished executing and will ascend to the parent context. Evaluating
    /// the if/while condition and moving to the appropriate body must be
    /// handled elsewhere.
    // fn find_next_control_point(
    //     &self,
    //     target: &ControlPoint,
    // ) -> NextControlPoint {
    //     let comp = target.comp;
    //     let comp_idx = self.env.cells[comp].as_comp().unwrap().comp_id;
    //     let root_ctrl = self.env.ctx.primary[comp_idx].control.expect(
    //         "Called `find_next_control_point` on a component with no control. This is an error, please report it",
    //     );

    //     // here's the goal:
    //     // we want to first walk the control tree for this component and find
    //     // the given control point and the path through the tree to get there.
    //     // Once we have this, we move back along the path to find the next
    //     // node to run (either a terminal node in an `invoke` or `enable` or a
    //     // non-terminal while/if condition check). Since this involves
    //     // backtracking, in the limit this means backtracking all the way to the
    //     // root of the component in which case the component has finished
    //     // executing.

    //     let cont = self.extract_next_search(root_ctrl);

    //     let mut search_stack = Vec::from([SearchNode {
    //         node: root_ctrl,
    //         next: cont,
    //     }]);

    //     while let Some(mut node) = search_stack.pop() {
    //         if node.node == target.control_leaf {
    //             // found the node! we return it to the stack which is now the
    //             // path from the root to our finished node
    //             search_stack.push(node);
    //             break;
    //         } else if let Some(next_node) = node.next.pop_front() {
    //             let next_search_node = SearchNode {
    //                 node: next_node,
    //                 next: self.extract_next_search(next_node),
    //             };
    //             // return the now modified original
    //             search_stack.push(node);
    //             // push the descendent next, so that the search continues
    //             // depth first
    //             search_stack.push(next_search_node);
    //         } else {
    //             // This node was not the one we wanted and none of its
    //             // children (if any) were either, we must return to the
    //             // parent which means dropping the node, i.e. doing nothing here
    //         }
    //     }

    //     if search_stack.is_empty() {
    //         // The reason this should never happen is that this implies a
    //         // controlpoint was constructed for a fully-structural component
    //         // instance which means something went wrong with the construction
    //         // as such an instance could not have a control program to reference
    //         panic!("Could not find control point in component, this should never happen. Please report this error.")
    //     }

    //     // phase two, backtrack to find the next node to run

    //     // remove the deepest node (i.e. our target)
    //     search_stack.pop();

    //     let mut immediate_next_node = None;

    //     while let Some(node) = search_stack.pop() {
    //         match &self.ctx().primary[node.node] {
    //             ControlNode::Seq(_) => {
    //                     if let Some(next) = node.next.get(0) {
    //                         // the target node will have been popped off the
    //                         // list during the search meaning the next node left
    //                         // over from the search is the next node to run
    //                          immediate_next_node = Some(*next);
    //                          // exit to descend the list
    //                          break;
    //                     } else {
    //                         // no next node, go to parent context
    //                     }
    //                 },
    //             ControlNode::Par(_) =>  {
    //                 // par arm needs to wait until all arms are finished
    //                 return NextControlPoint::FinishedParChild(ControlPoint::new(comp, node.node));
    //             },
    //             ControlNode::If(_) => {
    //                 // do nothing, go to parent context
    //             },
    //             ControlNode::While(_) => {
    //                 // need to recheck condition so the while itself is next
    //                 return NextControlPoint::Next(ControlPoint::new(comp, node.node));
    //             },
    //             //
    //             ControlNode::Empty(_)
    //             | ControlNode::Enable(_)
    //             | ControlNode::Invoke(_) => unreachable!("terminal nodes cannot be the parents of a node. If this happens something has gone horribly wrong and should be reported"),
    //         }
    //     }

    //     // phase 3, take the immediate next node and descend to find its leaf

    //     if let Some(node) = immediate_next_node {
    //         // we reuse the existing search stack without resetting it to allow
    //         // backtracking further if the immediate next node has no actual
    //         // leaves under it, e.g. a seq of empty seqs
    //         // TODO Griffin: double check this aspect later as it might
    //         // complicate things or introduce errors
    //         self.descend_to_leaf(node, &mut search_stack, comp)
    //     } else {
    //         // if we exit without finding the next node then it does not exist
    //         NextControlPoint::None
    //     }
    // }

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

    fn lookup_global_port_id(&self, port: GlobalPortRef) -> GlobalPortId {
        match port {
            GlobalPortRef::Port(p) => p,
            // TODO Griffin: Please make sure this error message is correct with
            // respect to the compiler
            GlobalPortRef::Ref(r) => self.env.ref_ports[r].expect("A ref port is being queried without a supplied ref-cell. This is an error?"),
        }
    }

    fn get_global_idx(
        &self,
        port: &PortRef,
        comp: GlobalCellId,
    ) -> GlobalPortId {
        let ledger = self.env.cells[comp].unwrap_comp();
        self.lookup_global_port_id(ledger.convert_to_global(port))
    }

    fn get_value(&self, port: &PortRef, comp: GlobalCellId) -> &Value {
        let port_idx = self.get_global_idx(port, comp);
        &self.env.ports[port_idx]
    }

    // fn descend_to_leaf(
    //     &self,
    //     // the node (possibly terminal) which we want to start evaluating
    //     immediate_next_node: ControlIdx,
    //     search_stack: &mut Vec<SearchNode>,
    //     comp: GlobalCellId,
    // ) -> NextControlPoint {
    //     search_stack.push(SearchNode {
    //         node: immediate_next_node,
    //         next: self.extract_next_search(immediate_next_node),
    //     });

    //     while let Some(mut node) = search_stack.pop() {
    //         match &self.ctx().primary[node.node] {
    //             ControlNode::Seq(_) => {
    //                 if let Some(next) = node.next.pop_front() {
    //                     search_stack.push(node);
    //                     let next_search_node = SearchNode {
    //                         node: next,
    //                         next: self.extract_next_search(next),
    //                     };
    //                     search_stack.push(next_search_node);
    //                 } else {
    //                     // this seq does not contain any more nodes.
    //                     // Currently only possible if the seq is empty
    //                 }
    //             },

    //             ControlNode::Par(p) => {
    //                 let mut ctrl_points = vec![];
    //                 let mut pars_activated = vec![];

    //                 let mut this_par = (ControlPoint::new(comp, node.node), p.stms().len() as u32);

    //                 // TODO Griffin: Maybe consider making this not
    //                 // recursive in the future
    //                 for arm in p.stms().iter().map( |x| {
    //                     self.descend_to_leaf(*x, &mut vec![], comp)
    //                 }) {
    //                     match arm {
    //                         NextControlPoint::None => {
    //                             this_par.1 -= 1;
    //                         },
    //                         NextControlPoint::Next(c) => ctrl_points.push(c),
    //                         NextControlPoint::FinishedParChild(_) => unreachable!("I think this impossible"),
    //                         NextControlPoint::StartedParChild(nodes, pars) => {
    //                             ctrl_points.extend(nodes);
    //                             pars_activated.extend(pars);
    //                         },
    //                     }
    //                 }

    //                 if this_par.1 != 0 {
    //                     pars_activated.push(this_par);
    //                     return NextControlPoint::StartedParChild(ctrl_points, pars_activated)
    //                 } else {
    //                     // there were no next nodes under this par, so we
    //                     // ascend the search tree and continue
    //                 }
    //             }

    //             // functionally terminals for the purposes of needing to be
    //             // seen in the control program and given extra treatment
    //             ControlNode::If(_)
    //             | ControlNode::While(_)
    //             // actual terminals
    //             | ControlNode::Invoke(_)
    //             | ControlNode::Enable(_)
    //             // might not want this here in the future, but makes sense
    //             // if we think about annotations on empty groups.
    //             | ControlNode::Empty(_)=> {
    //                 return NextControlPoint::Next(ControlPoint::new(comp, node.node))
    //             }
    //         }
    //     }
    //     NextControlPoint::None
    // }

    // may want to make this iterate directly if it turns out that the vec
    // allocation is too expensive in this context
    fn get_assignments(&self) -> AssignmentBundle {
        // maybe should give this a capacity equivalent to the number of
        // elements in the program counter? It would be a bit of an over
        // approximation
        let mut out = AssignmentBundle::new();
        for node in self.env.pc.iter() {
            match &self.ctx().primary[node.control_node] {
                ControlNode::Empty(_) => {
                    // don't need to add any assignments here
                }
                ControlNode::Enable(e) => {
                    out.push((node.comp, self.ctx().primary[e.group()].assignments))
                }

                ControlNode::Invoke(_) => todo!("invokes not yet implemented"),
                // The reason this shouldn't happen is that the program counter
                // should've processed these nodes into their children and
                // stored the needed auxillary data for par structures
                ControlNode::If(_) | ControlNode::While(_) => panic!("If/While nodes are present in the control program when `get_assignments` is called. This is an error, please report it."),
                ControlNode::Seq(_) | ControlNode::Par(_) => unreachable!(),
            }
        }

        out
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
                ControlNode::Enable(_) => todo!(),
                ControlNode::Invoke(_) => todo!("invokes not implemented yet"),
            }
        });

        // we want to iterate through all the nodes present in the program counter

        // first we need to check for conditional control nodes

        // self.simulate_combinational();

        Ok(())
    }

    fn evaluate_guard(&self, guard: GuardIdx, comp: GlobalCellId) -> bool {
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

    fn simulate_combinational(&mut self) {
        let assigns = self.get_assignments();
        let mut has_changed = true;

        // This is an upper-bound, i.e. if every assignment succeeds then there
        // will be this many entries
        let mut updates_vec: Vec<(GlobalPortId, Value)> =
            Vec::with_capacity(assigns.len());

        while has_changed {
            let updates = assigns.iter_over_assignments(self.ctx()).filter_map(
                |(comp_idx, assign)| {
                    if self.evaluate_guard(assign.guard, comp_idx) {
                        let val = self.get_value(&assign.src, comp_idx);
                        Some((
                            self.get_global_idx(&assign.dst, comp_idx),
                            val.clone(),
                        ))
                    } else {
                        None
                    }
                },
            );

            // want to buffer all updates before committing. It's not ideal to
            // be doing this in a tight loop.
            updates_vec.extend(updates);

            for (dest, val) in updates_vec.drain(..) {
                if self.env.ports[dest] != val {
                    has_changed = true
                }
                self.env.ports[dest] = val;
            }
        }
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
