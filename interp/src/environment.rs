//! Environment for interpreter.

use super::errors::{InterpreterError, InterpreterResult};
use super::interpreter::ComponentInterpreter;
use super::interpreter_ir as iir;
use super::primitives::{
    combinational, stateful, Entry, Primitive, Serializeable,
};
use super::stk_env::Smoosher;
use super::utils::AsRaw;
use super::utils::MemoryMap;
use super::values::Value;
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
type PrimitiveMap =
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
}

/// Helper functions for the environment.
impl InterpreterState {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init(
        ctx: &iir::ComponentCtx,
        target: &Rc<iir::Component>,
        mems: &Option<MemoryMap>,
    ) -> Self {
        Self {
            context: Rc::clone(ctx),
            clk: 0,
            port_map: InterpreterState::construct_port_map(&*target),
            cell_map: Self::construct_cell_map(target, &ctx, mems),
            component: target.clone(),
        }
    }

    /// Insert a new value for the given constant port into the environment
    pub fn insert<P: AsRaw<ir::Port>>(&mut self, port: P, value: Value) {
        self.port_map.set(port.as_raw(), value);
    }

    fn make_primitive(
        prim_name: &ir::Id,
        params: &ir::Binding,
        cell_name: Option<&ir::Id>,
        mems: &Option<MemoryMap>,
    ) -> Box<dyn Primitive> {
        match prim_name.as_ref() {
            "std_const" => Box::new(combinational::StdConst::new(params)),
            // unsigned and signed basic arith
            "std_add" | "std_sadd" | "std_fp_sadd" | "std_fp_add" => {
                Box::new(combinational::StdAdd::new(params))
            }
            "std_sub" | "std_ssub" | "std_fp_ssub" | "std_fp_sub" => {
                Box::new(combinational::StdSub::new(params))
            }
            // unsigned arith
            "std_mult_pipe" => {
                Box::new(stateful::StdMultPipe::<false>::new(params))
            }
            "std_div_pipe" => {
                Box::new(stateful::StdDivPipe::<false>::new(params))
            }
            // signed arith
            "std_smult_pipe" => {
                Box::new(stateful::StdMultPipe::<true>::new(params))
            }
            "std_sdiv_pipe" => {
                Box::new(stateful::StdMultPipe::<true>::new(params))
            }
            // fp unsigned arith
            "std_fp_mult_pipe" => {
                Box::new(stateful::StdFpMultPipe::<false>::new(params))
            }
            "std_fp_div_pipe" => {
                Box::new(stateful::StdFpDivPipe::<false>::new(params))
            }
            // fp signed arith
            "std_fp_smult_pipe" => {
                Box::new(stateful::StdFpMultPipe::<true>::new(params))
            }
            "std_fp_sdiv_pipe" => {
                Box::new(stateful::StdFpDivPipe::<true>::new(params))
            }
            // unsigned shifts
            "std_lsh" => Box::new(combinational::StdLsh::new(params)),
            "std_rsh" => Box::new(combinational::StdRsh::new(params)),
            // Logical operators
            "std_and" => Box::new(combinational::StdAnd::new(params)),
            "std_or" => Box::new(combinational::StdOr::new(params)),
            "std_xor" => Box::new(combinational::StdXor::new(params)),
            "std_not" => Box::new(combinational::StdNot::new(params)),
            // Unsigned Comparsion
            "std_ge" => Box::new(combinational::StdGe::new(params)),
            "std_le" => Box::new(combinational::StdLe::new(params)),
            "std_lt" => Box::new(combinational::StdLt::new(params)),
            "std_gt" => Box::new(combinational::StdGt::new(params)),
            "std_eq" => Box::new(combinational::StdEq::new(params)),
            "std_neq" => Box::new(combinational::StdNeq::new(params)),
            // Signed Comparison
            "std_sge" => Box::new(combinational::StdSge::new(params)),
            "std_sle" => Box::new(combinational::StdSle::new(params)),
            "std_slt" => Box::new(combinational::StdSlt::new(params)),
            "std_sgt" => Box::new(combinational::StdSgt::new(params)),
            "std_seq" => Box::new(combinational::StdSeq::new(params)),
            "std_sneq" => Box::new(combinational::StdSneq::new(params)),
            // unsigned FP comparison
            "std_fp_gt" => Box::new(combinational::StdFpGt::new(params)),
            // signed FP comparison
            "std_fp_sgt" => Box::new(combinational::StdFpSgt::new(params)),
            "std_fp_slt" => Box::new(combinational::StdFpSlt::new(params)),
            // Resizing ops
            "std_slice" => Box::new(combinational::StdSlice::new(params)),
            "std_pad" => Box::new(combinational::StdPad::new(params)),
            // State components
            "std_reg" => Box::new(stateful::StdReg::new(params)),
            "std_mem_d1" => {
                let mut prim = Box::new(stateful::StdMemD1::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals).unwrap();
                }
                prim
            }
            "std_mem_d2" => {
                let mut prim = Box::new(stateful::StdMemD2::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals).unwrap();
                }
                prim
            }
            "std_mem_d3" => {
                let mut prim = Box::new(stateful::StdMemD3::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals).unwrap();
                }
                prim
            }
            "std_mem_d4" => {
                let mut prim = Box::new(stateful::StdMemD4::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals).unwrap();
                }
                prim
            }

            p => panic!("Unknown primitive: {}", p),
        }
    }

    fn construct_cell_map(
        comp: &Rc<iir::Component>,
        ctx: &iir::ComponentCtx,
        mems: &Option<MemoryMap>,
    ) -> PrimitiveMap {
        let mut map = HashMap::new();
        for cell in comp.cells.iter() {
            let cl: &ir::Cell = &cell.borrow();

            match &cl.prototype {
                ir::CellType::Primitive {
                    name,
                    param_binding,
                    is_comb: _,
                } => {
                    let cell_name = match name.as_ref() {
                        "std_mem_d1" | "std_mem_d2" | "std_mem_d3"
                        | "std_mem_d4" => Some(cl.name()),
                        _ => None,
                    };

                    map.insert(
                        cl as ConstCell,
                        Self::make_primitive(
                            name,
                            param_binding,
                            cell_name,
                            mems,
                        ),
                    );
                }
                ir::CellType::Component { name } => {
                    let inner_comp =
                        ctx.iter().find(|x| x.name == name).unwrap();
                    let env = Self::init(ctx, inner_comp, mems);
                    let comp_interp: Box<dyn Primitive> = Box::new(
                        ComponentInterpreter::from_component(inner_comp, env),
                    );
                    map.insert(cl as ConstCell, comp_interp);
                }
                _ => {}
            }
        }
        Rc::new(RefCell::new(map))
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

    pub fn eval_guard(&self, guard: &ir::Guard) -> bool {
        match guard {
            ir::Guard::Or(g1, g2) => self.eval_guard(g1) || self.eval_guard(g2),
            ir::Guard::And(g1, g2) => {
                self.eval_guard(g1) && self.eval_guard(g2)
            }
            ir::Guard::Not(g) => !self.eval_guard(g),
            ir::Guard::Eq(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    == self.get_from_port(&g2.borrow())
            }
            ir::Guard::Neq(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    != self.get_from_port(&g2.borrow())
            }
            ir::Guard::Gt(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    > self.get_from_port(&g2.borrow())
            }
            ir::Guard::Lt(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    < self.get_from_port(&g2.borrow())
            }
            ir::Guard::Geq(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    >= self.get_from_port(&g2.borrow())
            }
            ir::Guard::Leq(g1, g2) => {
                self.get_from_port(&g1.borrow())
                    <= self.get_from_port(&g2.borrow())
            }
            ir::Guard::Port(p) => {
                let val = self.get_from_port(&p.borrow());
                if val.vec.len() != 1 {
                    panic!(
                        "Evaluating the truth value of a wire '{:?}' that is not one bit", p.borrow().canonical()
                    )
                } else {
                    val.as_u64() == 1
                }
            }
            ir::Guard::True => true,
        }
    }
}

impl Serialize for InterpreterState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let sv: StateView = self.into();
        sv.gen_serialzer().serialize(serializer)
    }
}
#[allow(clippy::borrowed_box)]
#[derive(Serialize, Clone)]
/// Struct to fully serialize the internal state of the environment
pub struct FullySerialize {
    ports: BTreeMap<ir::Id, BTreeMap<ir::Id, BTreeMap<ir::Id, Entry>>>,
    memories: BTreeMap<ir::Id, BTreeMap<ir::Id, Serializeable>>,
}

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
        self.gen_serialzer().serialize(serializer)
    }
}

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

    pub fn get_comp_name(&self) -> &ir::Id {
        match self {
            StateView::SingleView(c) => &c.component.name,
            StateView::Composite(c) => &c.0.component.name,
        }
    }

    /// Returns a string representing the current state of the environment. This
    /// just serializes the environment to a string and returns that string
    pub fn state_as_str(&self) -> String {
        serde_json::to_string_pretty(&self.gen_serialzer()).unwrap()
    }

    pub fn get_cells<S: AsRef<str> + Clone>(
        &self,
        name: &S,
    ) -> Vec<RRC<ir::Cell>> {
        let ctx_ref = self.get_ctx();
        ctx_ref.iter().filter_map(|x| x.find_cell(name)).collect()
    }

    pub fn gen_serialzer(&self) -> FullySerialize {
        let ctx = self.get_ctx();
        let cell_prim_map = &self.get_cell_map().borrow();

        let bmap: BTreeMap<_, _> = ctx
            .iter()
            .filter(|x| x.name == self.get_comp_name()) // there should only be one such comp
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
            .filter(|x| x.name == self.get_comp_name())
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
                                            cell.get_attribute("interp_signed")
                                                .is_some(),
                                        ),
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
