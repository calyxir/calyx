//! Environment for interpreter.

use super::interpreter::ComponentInterpreter;
use super::primitives::{combinational, stateful, Primitive, Serializeable};
use super::stk_env::Smoosher;
use super::utils::AsRaw;
use super::utils::MemoryMap;
use super::values::Value;
use super::RefHandler;
use calyx::ir::{self, RRC};
use serde::ser::SerializeMap;
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
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
type PrimitiveMap<'outer> =
    RRC<HashMap<ConstCell, Box<dyn crate::primitives::Primitive + 'outer>>>;

/// A map defining values for ports. As it is keyed by ConstPort, the lifetime of
/// the keys is independent of the ports. However as a result it is flat, rather
/// than heirarchical which simplifies the access interface.
type PortValMap = Smoosher<ConstPort, Value>;

/// The environment to interpret a Calyx program.
pub struct InterpreterState<'outer> {
    /// Clock count
    pub clk: u64,

    /// Mapping from cells to prims.
    pub cell_map: PrimitiveMap<'outer>,

    /// Use raw pointers for hashmap: ports to values
    // This is a Smoosher (see stk_env.rs)
    pub port_map: PortValMap,

    /// A reference to the context.
    pub context: ir::RRC<ir::Context>,
}

/// Helper functions for the environment.
impl<'outer> InterpreterState<'outer> {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init(
        ctx: ir::RRC<ir::Context>,
        target: &ir::Component,
        ref_handler: &'outer RefHandler<'outer>,
        mems: &Option<MemoryMap>,
    ) -> Self {
        Self {
            context: ctx.clone(),
            clk: 0,
            port_map: InterpreterState::construct_port_map(target),
            cell_map: Self::construct_cell_map(target, &ctx, ref_handler, mems),
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
            "std_add" => Box::new(combinational::StdAdd::new(params)),
            "std_sub" => Box::new(combinational::StdSub::new(params)),
            "std_lsh" => Box::new(combinational::StdLsh::new(params)),
            "std_rsh" => Box::new(combinational::StdRsh::new(params)),
            "std_and" => Box::new(combinational::StdAnd::new(params)),
            "std_or" => Box::new(combinational::StdOr::new(params)),
            "std_xor" => Box::new(combinational::StdXor::new(params)),
            "std_ge" => Box::new(combinational::StdGe::new(params)),
            "std_le" => Box::new(combinational::StdLe::new(params)),
            "std_lt" => Box::new(combinational::StdLt::new(params)),
            "std_gt" => Box::new(combinational::StdGt::new(params)),
            "std_eq" => Box::new(combinational::StdEq::new(params)),
            "std_neq" => Box::new(combinational::StdNeq::new(params)),
            "std_not" => Box::new(combinational::StdNot::new(params)),
            "std_slice" => Box::new(combinational::StdSlice::new(params)),
            "std_pad" => Box::new(combinational::StdPad::new(params)),
            "std_reg" => Box::new(stateful::StdReg::new(params)),
            "std_mult_pipe" => Box::new(stateful::StdMultPipe::new(params)),
            "std_div_pipe" => Box::new(stateful::StdDivPipe::new(params)),
            "std_const" => Box::new(combinational::StdConst::new(params)),
            "std_mem_d1" => {
                let mut prim = Box::new(stateful::StdMemD1::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals);
                }
                prim
            }
            "std_mem_d2" => {
                let mut prim = Box::new(stateful::StdMemD2::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals);
                }
                prim
            }
            "std_mem_d3" => {
                let mut prim = Box::new(stateful::StdMemD3::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals);
                }
                prim
            }
            "std_mem_d4" => {
                let mut prim = Box::new(stateful::StdMemD4::new(params));

                let init = mems
                    .as_ref()
                    .and_then(|x| cell_name.and_then(|name| x.get(name)));

                if let Some(vals) = init {
                    prim.initialize_memory(vals);
                }
                prim
            }

            p => panic!("Unknown primitive: {}", p),
        }
    }

    fn construct_cell_map(
        comp: &ir::Component,
        ctx: &ir::RRC<ir::Context>,
        handler: &'outer RefHandler<'outer>,
        mems: &Option<MemoryMap>,
    ) -> PrimitiveMap<'outer> {
        let mut map = HashMap::new();
        for cell in comp.cells.iter() {
            let cl: &ir::Cell = &cell.borrow();

            match &cl.prototype {
                ir::CellType::Primitive {
                    name,
                    param_binding,
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
                    let (comp, control) = handler.get_by_name(name);
                    let env = Self::init(ctx.clone(), comp, handler, mems);
                    let comp_interp: Box<dyn Primitive> =
                        Box::new(ComponentInterpreter::from_component(
                            comp, control, env,
                        ));
                    map.insert(cl as ConstCell, comp_interp);
                }
                _ => {}
            }
        }
        Rc::new(RefCell::new(map))
    }

    fn construct_port_map(comp: &ir::Component) -> PortValMap {
        let mut map = HashMap::new();

        for port in comp.signature.borrow().ports.iter() {
            let pt: &ir::Port = &port.borrow();
            map.insert(pt as ConstPort, Value::bit_low());
        }
        for group in comp.groups.iter() {
            let grp = group.borrow();
            for hole in &grp.holes {
                let pt: &ir::Port = &hole.borrow();
                map.insert(pt as ConstPort, Value::bit_low());
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
                        map.insert(
                            pt as ConstPort,
                            Value::from(*val, *width).unwrap(),
                        );
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
                            )
                            .unwrap(),
                        );
                    }
                }
                ir::CellType::Component { .. } => {
                    for port in &cll.ports {
                        let pt: &ir::Port = &port.borrow();
                        map.insert(pt as ConstPort, Value::from(0, 0).unwrap());
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

    /// Returns a string representing the current state of the environment. This
    /// just serializes the environment to a string and returns that string
    pub fn state_as_str(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
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
        }
    }

    /// Merge the given environments. Must be called from the root environment
    pub fn merge_many(mut self, others: Vec<Self>) -> Self {
        let clk = others
            .iter()
            .chain(once(&self))
            .map(|x| x.clk)
            .max()
            .unwrap(); // safe because of once

        self.port_map = self
            .port_map
            .merge_many(others.into_iter().map(|x| x.port_map).collect());
        self.clk = clk;

        self
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

    pub fn get_cell<S: AsRef<str> + Clone>(
        &self,
        name: &S,
    ) -> Vec<RRC<ir::Cell>> {
        let ctx_ref = self.context.borrow();
        ctx_ref
            .components
            .iter()
            .filter_map(|x| x.find_cell(name))
            .collect()
    }
}

impl<'outer> Serialize for InterpreterState<'outer> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ctx: &ir::Context = &self.context.borrow();
        let cell_prim_map = &self.cell_map.borrow();

        let bmap: BTreeMap<_, _> = ctx
            .components
            .iter()
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
                                (
                                    port.borrow().name.clone(),
                                    self.get_from_port(&port.borrow()).as_u64(),
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
            .components
            .iter()
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
                                        Primitive::serialize(&**prim),
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

        let p = FullySerialize {
            ports: bmap,
            memories: cell_map,
        };
        p.serialize(serializer)
    }
}
#[allow(clippy::borrowed_box)]
#[derive(Serialize)]
/// Struct to fully serialize the internal state of the environment
struct FullySerialize {
    ports: BTreeMap<ir::Id, BTreeMap<ir::Id, BTreeMap<ir::Id, u64>>>,
    memories: BTreeMap<ir::Id, BTreeMap<ir::Id, Serializeable>>,
}
