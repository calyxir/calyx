//! Environment for interpreter.

use super::names::{
    ComponentQualifiedInstanceName, GroupQIN, InstanceName,
    QualifiedInstanceName,
};
use super::stk_env::Smoosher;
use crate::configuration::Config;
use crate::debugger::name_tree::ActiveTreeNode;
use crate::debugger::PrintCode;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::interpreter::ComponentInterpreter;
use crate::interpreter::Interpreter;
use crate::interpreter_ir as iir;
use crate::primitives::{
    combinational, stateful, Entry, Primitive, Serializeable,
};
use crate::utils::AsRaw;
use crate::utils::MemoryMap;
use crate::values::Value;
use calyx::ir::{self, RRC};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::iter::once;
use std::rc::Rc;

/// A raw pointer reference to a cell. Can only be used as a key, but cannot be
/// used to access the cell itself.
type ConstCell = *const ir::Cell;

/// A raw pointer reference to a port. As with cell, it is only suitable for use
/// as a key and cannot be used to access the port itself.
type ConstPort = *const ir::Port;

/// A map defining primitive implementations for Cells. As it is keyed by
/// ConstCell the lifetime of the keys is independent of the actual cells.
pub(crate) type PrimitiveMap =
    RRC<HashMap<ConstCell, Box<dyn crate::primitives::Primitive>>>;

/// A map defining values for ports. As it is keyed by ConstPort, the lifetime of
/// the keys is independent of the ports. However as a result it is flat, rather
/// than heirarchical which simplifies the access interface.
type PortValMap = Smoosher<ConstPort, Value>;

/// The environment to interpret a Calyx program.
pub struct InterpreterState {
    /// Clock count
    pub clk: u64,

    /// Mapping from cells to prims.
    pub cell_map: PrimitiveMap,

    /// Use raw pointers for hashmap: ports to values
    // This is a Smoosher (see stk_env.rs)
    pub port_map: PortValMap,

    /// An rc handle to a vec of components
    pub context: iir::ComponentCtx,

    /// The name of the component this environment is for. Used for printing the
    /// environment state.
    pub component: Rc<iir::Component>,

    pub sub_comp_set: Rc<HashSet<ConstCell>>,

    allow_par_conflicts: bool,
}

/// Helper functions for the environment.
impl InterpreterState {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init_top_level(
        ctx: &iir::ComponentCtx,
        target: &Rc<iir::Component>,
        mems: &Option<MemoryMap>,
        configs: &Config,
    ) -> InterpreterResult<Self> {
        // only for the main component
        let qin =
            ComponentQualifiedInstanceName::new_single(target, &target.name);
        let (map, set) =
            Self::construct_cell_map(target, ctx, mems, &qin, configs)?;

        Ok(Self {
            context: Rc::clone(ctx),
            clk: 0,
            port_map: InterpreterState::construct_port_map(&*target),
            cell_map: map,
            component: target.clone(),
            sub_comp_set: Rc::new(set),
            allow_par_conflicts: configs.allow_par_conflicts,
        })
    }

    pub fn init(
        ctx: &iir::ComponentCtx,
        target: &Rc<iir::Component>,
        mems: &Option<MemoryMap>,
        qin: &ComponentQualifiedInstanceName,
        configs: &Config,
    ) -> InterpreterResult<Self> {
        let (map, set) =
            Self::construct_cell_map(target, ctx, mems, qin, configs)?;

        Ok(Self {
            context: Rc::clone(ctx),
            clk: 0,
            port_map: InterpreterState::construct_port_map(&*target),
            cell_map: map,
            component: target.clone(),
            sub_comp_set: Rc::new(set),
            allow_par_conflicts: configs.allow_par_conflicts,
        })
    }

    /// Insert a new value for the given constant port into the environment
    pub fn insert<P: AsRaw<ir::Port>>(&mut self, port: P, value: Value) {
        self.port_map.set(port.as_raw(), value);
    }

    fn make_primitive(
        prim_name: &ir::Id,
        params: &ir::Binding,
        cell_name: &ir::Id,
        mems: &Option<MemoryMap>,
        qin_name: &ComponentQualifiedInstanceName,
        configs: &Config,
    ) -> InterpreterResult<Box<dyn Primitive>> {
        let cell_qin = QualifiedInstanceName::new(qin_name, cell_name).as_id();
        Ok(match prim_name.as_ref() {
            "std_const" => {
                Box::new(combinational::StdConst::new(params, cell_qin))
            }
            // unsigned and signed basic arith
            "std_add" | "std_sadd" => Box::new(combinational::StdAdd::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            "std_sub" | "std_ssub" => Box::new(combinational::StdSub::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            // fp basic arith
            "std_fp_sadd" | "std_fp_add" => {
                Box::new(combinational::StdFpAdd::new(
                    params,
                    cell_qin,
                    configs.error_on_overflow,
                ))
            }
            "std_fp_ssub" | "std_fp_sub" => {
                Box::new(combinational::StdFpSub::new(
                    params,
                    cell_qin,
                    configs.error_on_overflow,
                ))
            }
            // unsigned arith
            "std_mult_pipe" => Box::new(stateful::StdMultPipe::<false>::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            "std_div_pipe" => Box::new(stateful::StdDivPipe::<false>::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            // signed arith
            "std_smult_pipe" => Box::new(stateful::StdMultPipe::<true>::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            "std_sdiv_pipe" => Box::new(stateful::StdDivPipe::<true>::new(
                params,
                cell_qin,
                configs.error_on_overflow,
            )),
            // fp unsigned arith
            "std_fp_mult_pipe" => Box::new(
                stateful::StdFpMultPipe::<false>::new(params, cell_qin),
            ),
            "std_fp_div_pipe" => {
                Box::new(stateful::StdFpDivPipe::<false>::new(params, cell_qin))
            }
            // fp signed arith
            "std_fp_smult_pipe" => {
                Box::new(stateful::StdFpMultPipe::<true>::new(params, cell_qin))
            }
            "std_fp_sdiv_pipe" => {
                Box::new(stateful::StdFpDivPipe::<true>::new(params, cell_qin))
            }
            // unsigned shifts
            "std_lsh" => Box::new(combinational::StdLsh::new(params, cell_qin)),
            "std_rsh" => Box::new(combinational::StdRsh::new(params, cell_qin)),
            // Logical operators
            "std_and" => Box::new(combinational::StdAnd::new(params, cell_qin)),
            "std_or" => Box::new(combinational::StdOr::new(params, cell_qin)),
            "std_xor" => Box::new(combinational::StdXor::new(params, cell_qin)),
            "std_not" => Box::new(combinational::StdNot::new(params, cell_qin)),
            "std_wire" => {
                Box::new(combinational::StdWire::new(params, cell_qin))
            }
            // Unsigned Comparsion
            "std_ge" => Box::new(combinational::StdGe::new(params, cell_qin)),
            "std_le" => Box::new(combinational::StdLe::new(params, cell_qin)),
            "std_lt" => Box::new(combinational::StdLt::new(params, cell_qin)),
            "std_gt" => Box::new(combinational::StdGt::new(params, cell_qin)),
            "std_eq" => Box::new(combinational::StdEq::new(params, cell_qin)),
            "std_neq" => Box::new(combinational::StdNeq::new(params, cell_qin)),
            // Signed Comparison
            "std_sge" => Box::new(combinational::StdSge::new(params, cell_qin)),
            "std_sle" => Box::new(combinational::StdSle::new(params, cell_qin)),
            "std_slt" => Box::new(combinational::StdSlt::new(params, cell_qin)),
            "std_sgt" => Box::new(combinational::StdSgt::new(params, cell_qin)),
            "std_seq" => Box::new(combinational::StdSeq::new(params, cell_qin)),
            "std_sneq" => {
                Box::new(combinational::StdSneq::new(params, cell_qin))
            }
            // unsigned FP comparison
            "std_fp_gt" => {
                Box::new(combinational::StdFpGt::new(params, cell_qin))
            }
            // signed FP comparison
            "std_fp_sgt" => {
                Box::new(combinational::StdFpSgt::new(params, cell_qin))
            }
            "std_fp_slt" => {
                Box::new(combinational::StdFpSlt::new(params, cell_qin))
            }
            // Resizing ops
            "std_slice" => {
                Box::new(combinational::StdSlice::new(params, cell_qin))
            }
            "std_pad" => Box::new(combinational::StdPad::new(params, cell_qin)),
            // State components
            "std_reg" => Box::new(stateful::StdReg::new(params, cell_qin)),
            "std_mem_d1" => {
                let mut prim = Box::new(stateful::StdMemD1::new(
                    params,
                    cell_qin,
                    configs.allow_invalid_memory_access,
                ));

                let init = mems.as_ref().and_then(|x| x.get(cell_name));

                if let Some(vals) = init {
                    prim.initialize_memory(vals)?;
                }
                prim
            }
            "std_mem_d2" => {
                let mut prim = Box::new(stateful::StdMemD2::new(
                    params,
                    cell_qin,
                    configs.allow_invalid_memory_access,
                ));

                let init = mems.as_ref().and_then(|x| x.get(cell_name));

                if let Some(vals) = init {
                    prim.initialize_memory(vals)?;
                }
                prim
            }
            "std_mem_d3" => {
                let mut prim = Box::new(stateful::StdMemD3::new(
                    params,
                    cell_qin,
                    configs.allow_invalid_memory_access,
                ));

                let init = mems.as_ref().and_then(|x| x.get(cell_name));

                if let Some(vals) = init {
                    prim.initialize_memory(vals)?;
                }
                prim
            }
            "std_mem_d4" => {
                let mut prim = Box::new(stateful::StdMemD4::new(
                    params,
                    cell_qin,
                    configs.allow_invalid_memory_access,
                ));

                let init = mems.as_ref().and_then(|x| x.get(cell_name));

                if let Some(vals) = init {
                    prim.initialize_memory(vals)?;
                }
                prim
            }

            p => return Err(InterpreterError::UnknownPrimitive(p.to_string())),
        })
    }

    fn construct_cell_map(
        comp: &Rc<iir::Component>,
        ctx: &iir::ComponentCtx,
        mems: &Option<MemoryMap>,
        qin_name: &ComponentQualifiedInstanceName,
        configs: &Config,
    ) -> InterpreterResult<(PrimitiveMap, HashSet<ConstCell>)> {
        let mut map = HashMap::new();
        let mut set = HashSet::new();
        for cell in comp.cells.iter() {
            let cl: &ir::Cell = &cell.borrow();

            match &cl.prototype {
                ir::CellType::Primitive {
                    name,
                    param_binding,
                    is_comb: _,
                } => {
                    map.insert(
                        cl as ConstCell,
                        Self::make_primitive(
                            name,
                            param_binding,
                            cl.name(),
                            mems,
                            qin_name,
                            configs,
                        )?,
                    );
                }
                ir::CellType::Component { name } => {
                    let inner_comp =
                        ctx.iter().find(|x| x.name == name).unwrap();
                    let qin = qin_name
                        .new_extend(InstanceName::new(inner_comp, cl.name()));
                    let env = Self::init(ctx, inner_comp, mems, &qin, configs)?;
                    let comp_interp: Box<dyn Primitive> =
                        Box::new(ComponentInterpreter::from_component(
                            inner_comp, env, qin,
                        ));
                    set.insert(cl.as_raw());
                    map.insert(cl as ConstCell, comp_interp);
                }
                _ => {}
            }
        }
        Ok((Rc::new(RefCell::new(map)), set))
    }

    fn construct_port_map(comp: &iir::Component) -> PortValMap {
        let mut map = HashMap::new();

        for port in comp.signature.borrow().ports.iter() {
            let pt: &ir::Port = &port.borrow();
            map.insert(pt as ConstPort, Value::zeroes(pt.width as usize));
        }
        for group in comp.groups.iter() {
            let grp = group.borrow();
            for hole in &grp.holes {
                let pt: &ir::Port = &hole.borrow();
                map.insert(pt as ConstPort, Value::zeroes(pt.width as usize));
            }
        }
        for cell in comp.cells.iter() {
            //also iterate over groups cuz they also have ports
            //iterate over ports, getting their value and putting into map
            let cll = cell.borrow();
            match &cll.prototype {
                ir::CellType::Constant { val, width } => {
                    for port in &cll.ports {
                        let pt: &ir::Port = &port.borrow();
                        map.insert(pt as ConstPort, Value::from(*val, *width));
                    }
                }
                ir::CellType::Primitive { .. } => {
                    for port in &cll.ports {
                        let pt: &ir::Port = &port.borrow();
                        map.insert(
                            pt as ConstPort,
                            Value::from(
                                cll.get_parameter("VALUE").unwrap_or_default(),
                                pt.width,
                            ),
                        );
                    }
                }
                ir::CellType::Component { .. } => {
                    for port in &cll.ports {
                        let pt: &ir::Port = &port.borrow();
                        map.insert(
                            pt as ConstPort,
                            Value::zeroes(pt.width as usize),
                        );
                    }
                }
                _ => unreachable!(),
            }
        }

        map.into()
    }

    /// Return the value associated with a component's port.
    pub fn get_from_port<P: AsRaw<ir::Port>>(&self, port: P) -> &Value {
        self.port_map.get(&port.as_raw()).unwrap()
    }

    /// Outputs the cell state;
    // TODO (write to a specified output in the future) We could do the printing
    // of values here for tracing purposes as discussed. Could also have a
    // separate DS that we could put the cell states into for more custom tracing
    pub fn print_env(&self) {
        println!("{}", serde_json::to_string_pretty(&self).unwrap());
    }

    pub fn print_env_raw(&self) {
        let sv: StateView = self.into();
        println!(
            "{}",
            serde_json::to_string_pretty(&sv.gen_serialzer(true)).unwrap()
        );
    }
    /// A predicate that checks if the given cell points to a combinational
    /// primitive (or component?)
    pub fn cell_is_comb<C: AsRaw<ir::Cell>>(&self, cell: C) -> bool {
        self.cell_map
            .borrow()
            .get(&cell.as_raw())
            .unwrap()
            .is_comb()
    }

    /// Creates a fork of the source environment which has the same clock and
    /// underlying primitive map but whose stack environment has been forked
    /// from the source's stack environment allowing divergence from the fork
    /// point
    pub fn fork(&mut self) -> Self {
        let other_pv_map = if self.port_map.top().is_empty() {
            self.port_map.fork_from_tail()
        } else {
            self.port_map.fork()
        };
        Self {
            clk: self.clk,
            cell_map: self.cell_map.clone(),
            port_map: other_pv_map,
            context: Rc::clone(&self.context),
            component: self.component.clone(),
            sub_comp_set: Rc::clone(&self.sub_comp_set),
            allow_par_conflicts: self.allow_par_conflicts,
        }
    }
    /// Creates a fork of the source environment which has the same clock and
    /// underlying primitive map but whose stack environment has been forked
    /// from the source's stack environment allowing divergence from the fork
    /// point. This forces the creation of a new layer, unlike fork
    pub fn force_fork(&mut self) -> Self {
        Self {
            clk: self.clk,
            cell_map: self.cell_map.clone(),
            port_map: self.port_map.fork(),
            context: Rc::clone(&self.context),
            component: self.component.clone(),
            sub_comp_set: Rc::clone(&self.sub_comp_set),
            allow_par_conflicts: self.allow_par_conflicts,
        }
    }

    /// Merge the given environments. Must be called from the root environment
    pub fn merge_many(
        mut self,
        others: Vec<Self>,
        overlap: &HashSet<*const ir::Port>,
    ) -> InterpreterResult<Self> {
        let clk = others
            .iter()
            .chain(once(&self))
            .map(|x| x.clk)
            .max()
            .unwrap(); // safe because of once

        let port_map = self.port_map;
        let merged = port_map.merge_many(
            others.into_iter().map(|x| x.port_map).collect(),
            overlap,
            self.allow_par_conflicts,
        );

        self.port_map = match merged {
            Ok(ok) => Ok(ok),
            Err(e) => {
                let mut ie: InterpreterError = e.into();
                if let InterpreterError::ParOverlap { parent_id, .. } = &mut ie
                {
                    // this is just to make the error point toward the component, rather
                    // than printing "_this"
                    if parent_id == "_this" {
                        *parent_id = self.component.name.clone()
                    }
                }
                Err(ie)
            }
        }?;

        self.clk = clk;

        Ok(self)
    }

    pub fn eval_guard(&self, guard: &ir::Guard) -> InterpreterResult<bool> {
        Ok(match guard {
            ir::Guard::Or(g1, g2) => {
                self.eval_guard(g1)? || self.eval_guard(g2)?
            }
            ir::Guard::And(g1, g2) => {
                self.eval_guard(g1)? && self.eval_guard(g2)?
            }
            ir::Guard::Not(g) => !self.eval_guard(g)?,
            ir::Guard::CompOp(op, g1, g2) => {
                let p1 = self.get_from_port(&g1.borrow());
                let p2 = self.get_from_port(&g2.borrow());
                match op {
                    ir::PortComp::Eq => p1 == p2,
                    ir::PortComp::Neq => p1 != p2,
                    ir::PortComp::Gt => p1 > p2,
                    ir::PortComp::Lt => p1 < p2,
                    ir::PortComp::Geq => p1 >= p2,
                    ir::PortComp::Leq => p1 <= p2,
                }
            }
            ir::Guard::Port(p) => {
                let val = self.get_from_port(&p.borrow());
                if val.len() != 1 {
                    let can = p.borrow().canonical();
                    return Err(InterpreterError::InvalidBoolCast(
                        (can.0, can.1),
                        p.borrow().width,
                    ));
                } else {
                    val.as_bool()
                }
            }
            ir::Guard::True => true,
        })
    }

    pub fn sub_component_currently_executing(&self) -> HashSet<GroupQIN> {
        let lookup = self.cell_map.borrow();

        self.sub_comp_set
            .iter()
            .map(|x| {
                lookup[x]
                    .get_comp_interpreter()
                    .unwrap()
                    .currently_executing_group()
            })
            .flatten()
            .collect()
    }

    pub fn as_state_view(&self) -> StateView<'_> {
        StateView::SingleView(self)
    }
    pub fn get_active_tree(&self) -> Vec<ActiveTreeNode> {
        let lookup = self.cell_map.borrow();

        self.sub_comp_set
            .iter()
            .map(|x| {
                lookup[x].get_comp_interpreter().unwrap().get_active_tree()
            })
            .flatten()
            .collect()
    }
}

impl Serialize for InterpreterState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let sv: StateView = self.into();
        sv.gen_serialzer(false).serialize(serializer)
    }
}
#[allow(clippy::borrowed_box)]
#[derive(Serialize, Clone)]
/// Struct to fully serialize the internal state of the environment
pub struct FullySerialize {
    ports: BTreeMap<ir::Id, BTreeMap<ir::Id, BTreeMap<ir::Id, Entry>>>,
    memories: BTreeMap<ir::Id, BTreeMap<ir::Id, Serializeable>>,
}

#[derive(Clone)]
pub struct CompositeView<'a>(&'a InterpreterState, Vec<StateView<'a>>);

impl<'a, 'outer> CompositeView<'a> {
    pub fn new(state: &'a InterpreterState, vec: Vec<StateView<'a>>) -> Self {
        Self(state, vec)
    }
}

impl<'a> Serialize for StateView<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.gen_serialzer(false).serialize(serializer)
    }
}

#[derive(Clone)]
pub enum StateView<'inner> {
    SingleView(&'inner InterpreterState),
    Composite(CompositeView<'inner>),
}

impl<'a, 'outer> From<&'a InterpreterState> for StateView<'a> {
    fn from(env: &'a InterpreterState) -> Self {
        Self::SingleView(env)
    }
}

impl<'a> From<CompositeView<'a>> for StateView<'a> {
    fn from(cv: CompositeView<'a>) -> Self {
        Self::Composite(cv)
    }
}

impl<'a> StateView<'a> {
    pub fn lookup<P: AsRaw<ir::Port>>(&self, target: P) -> &Value {
        match self {
            StateView::SingleView(sv) => sv.get_from_port(target),
            StateView::Composite(cv) => match cv.1.len() {
                0 => cv.0.get_from_port(target),
                1 => cv.1[0].lookup(target),
                _ => {
                    let original = cv.0.get_from_port(target.as_raw());
                    let new =
                        cv.1.iter()
                            .filter_map(|x| {
                                let val = x.lookup(target.as_raw());
                                if val == original {
                                    None
                                } else {
                                    Some(val)
                                }
                            })
                            .collect::<Vec<_>>();
                    match new.len() {
                        0 => original,
                        1 => new[0],
                        _ => panic!("conflicting parallel values"),
                    }
                }
            },
        }
    }

    pub fn sub_component_currently_executing(&self) -> HashSet<GroupQIN> {
        match self {
            StateView::SingleView(sv) => sv.sub_component_currently_executing(),
            StateView::Composite(c) => c.0.sub_component_currently_executing(),
        }
    }

    pub fn get_ctx(&self) -> &iir::ComponentCtx {
        match self {
            StateView::SingleView(sv) => &sv.context,
            StateView::Composite(cv) => &cv.0.context,
        }
    }

    pub fn get_cell_map(&self) -> &PrimitiveMap {
        match self {
            StateView::SingleView(sv) => &sv.cell_map,
            StateView::Composite(cv) => &cv.0.cell_map,
        }
    }

    pub fn get_comp(&self) -> &Rc<iir::Component> {
        match self {
            StateView::SingleView(c) => &c.component,
            StateView::Composite(c) => &c.0.component,
        }
    }
    pub fn get_active_tree(&self) -> Vec<ActiveTreeNode> {
        match self {
            StateView::SingleView(c) => c.get_active_tree(),
            StateView::Composite(c) => c.0.get_active_tree(),
        }
    }

    pub fn get_cell_state<R: AsRaw<ir::Cell>>(
        &self,
        cell: R,
        print_code: &PrintCode,
    ) -> Serializeable {
        let map = self.get_cell_map();
        let map_ref = map.borrow();
        map_ref
            .get(&cell.as_raw())
            .map(|x| Primitive::serialize(&**x, Some(*print_code)))
            .unwrap_or(Serializeable::Empty)
    }

    /// Returns a string representing the current state of the environment. This
    /// just serializes the environment to a string and returns that string
    pub fn state_as_str(&self) -> String {
        serde_json::to_string_pretty(&self.gen_serialzer(false)).unwrap()
    }

    pub fn get_cells<S: AsRef<str> + Clone>(
        &self,
        name: &S,
    ) -> Vec<RRC<ir::Cell>> {
        let ctx_ref = self.get_ctx();
        ctx_ref.iter().filter_map(|x| x.find_cell(name)).collect()
    }

    pub fn get_cell<S: AsRef<str> + Clone>(
        &self,
        name: S,
    ) -> Option<RRC<ir::Cell>> {
        match self {
            StateView::SingleView(sv) => sv.component.find_cell(&name),
            StateView::Composite(cv) => cv.0.component.find_cell(&name),
        }
    }

    pub fn gen_serialzer(&self, raw: bool) -> FullySerialize {
        let ctx = self.get_ctx();
        let cell_prim_map = &self.get_cell_map().borrow();

        let bmap: BTreeMap<_, _> = ctx
            .iter()
            .filter(|x| x.name == self.get_comp().name) // there should only be one such comp
            .map(|comp| {
                let inner_map: BTreeMap<_, _> = comp
                    .cells
                    .iter()
                    .map(|cell| {
                        let inner_map: BTreeMap<_, _> = cell
                            .borrow()
                            .ports
                            .iter()
                            .map(|port| {
                                let value = self.lookup(port.as_raw());

                                (
                                    port.borrow().name.clone(),
                                    if port
                                        .borrow()
                                        .attributes
                                        .has("interp_signed")
                                    {
                                        value.as_i64().into()
                                    } else {
                                        value.as_u64().into()
                                    },
                                )
                            })
                            .collect();
                        (cell.borrow().name().clone(), inner_map)
                    })
                    .collect();
                (comp.name.clone(), inner_map)
            })
            .collect();
        let cell_map: BTreeMap<_, _> = ctx
            .iter()
            .filter(|x| x.name == self.get_comp().name)
            .map(|comp| {
                let inner_map: BTreeMap<_, _> = comp
                    .cells
                    .iter()
                    .filter_map(|cell_ref| {
                        let cell = cell_ref.borrow();
                        if cell.get_attribute("external").is_some() {
                            if let Some(prim) = cell_prim_map
                                .get(&(&cell as &ir::Cell as ConstCell))
                            {
                                if !prim.is_comb() {
                                    return Some((
                                        cell.name().clone(),
                                        Primitive::serialize(
                                            &**prim,
                                            raw.then(|| PrintCode::Binary),
                                        ), //TODO Griffin: Fix this
                                    ));
                                }
                            }
                        }
                        None
                    })
                    .collect();
                (comp.name.clone(), inner_map)
            })
            .collect();

        FullySerialize {
            ports: bmap,
            memories: cell_map,
        }
    }
}

pub struct MutCompositeView<'a>(
    &'a mut InterpreterState,
    Vec<MutStateView<'a>>,
);

pub enum MutStateView<'inner> {
    Single(&'inner mut InterpreterState),
    Composite(MutCompositeView<'inner>),
}

impl<'inner> MutCompositeView<'inner> {
    pub fn new(
        state: &'inner mut InterpreterState,
        vec: Vec<MutStateView<'inner>>,
    ) -> Self {
        Self(state, vec)
    }
    pub fn insert<P: AsRaw<ir::Port>>(&mut self, port: P, value: Value) {
        let raw = port.as_raw();
        self.0.insert(raw, value.clone());
        for view in self.1.iter_mut() {
            view.insert(raw, value.clone())
        }
    }
}

impl<'a> From<&'a mut InterpreterState> for MutStateView<'a> {
    fn from(env: &'a mut InterpreterState) -> Self {
        Self::Single(env)
    }
}

impl<'a> From<MutCompositeView<'a>> for MutStateView<'a> {
    fn from(mv: MutCompositeView<'a>) -> Self {
        Self::Composite(mv)
    }
}

impl<'a> MutStateView<'a> {
    pub fn insert<P: AsRaw<ir::Port>>(&mut self, port: P, value: Value) {
        match self {
            MutStateView::Single(s) => s.insert(port, value),
            MutStateView::Composite(c) => c.insert(port, value),
        }
    }
}

pub trait State {
    fn lookup(&self, target: &*const ir::Port) -> &Value;
    fn state_as_str(&self) -> String;
}

impl<'a> State for StateView<'a> {
    fn lookup(&self, target: &*const ir::Port) -> &Value {
        StateView::lookup(self, *target)
    }

    fn state_as_str(&self) -> String {
        StateView::state_as_str(self)
    }
}
