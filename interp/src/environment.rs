//! Environment for interpreter.

use super::primitives::{combinational, stateful, Primitive};
use super::stk_env::Smoosher;
use super::utils::MemoryMap;
use super::values::Value;
use calyx::ir::{self, RRC};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// A raw pointer reference to a cell. Can only be used as a key, but cannot be
/// used to access the cell itself
type ConstCell = *const ir::Cell;

/// A raw pointer reference to a port. As with cell, it is only suitable for use
/// as a key and cannot be used to access the port itself
type ConstPort = *const ir::Port;

/// A map defining primitive implementations for Cells. As it is keyed by
/// CellRefs the lifetime of the keys is independent of the actual cells
type PrimitiveMap =
    RRC<HashMap<ConstCell, Box<dyn crate::primitives::Primitive>>>;

/// A map defining values for ports. As it is keyed by PortRefs, the lifetime of
/// the keys is independent of the ports. However as a result it is flat, rather
/// than heirarchical which simplifies the access interface
type PortValMap = Smoosher<ConstPort, Value>;

/// The environment to interpret a Calyx program.
pub struct InterpreterState {
    ///clock count
    pub clk: u64,

    ///mapping from cells to prims
    pub cell_prim_map: PrimitiveMap,

    ///use raw pointers for hashmap: ports to values
    //this is a Smoosher (see stk_env.rs)
    pub pv_map: PortValMap,

    /// A reference to the context.
    pub context: ir::RRC<ir::Context>,
}

/// Helper functions for the environment.
impl InterpreterState {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init(ctx: &ir::RRC<ir::Context>, mems: &Option<MemoryMap>) -> Self {
        Self {
            context: ctx.clone(),
            clk: 0,
            pv_map: InterpreterState::construct_pv_map(&ctx.borrow()),
            cell_prim_map: Self::construct_cp_map(&ctx.borrow(), mems),
        }
    }

    pub fn insert(&mut self, port: ConstPort, value: Value) {
        self.pv_map.set(port, value);
    }

    fn make_primitive(
        prim_name: ir::Id,
        params: ir::Binding,
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

    fn construct_cp_map(
        ctx: &ir::Context,
        mems: &Option<MemoryMap>,
    ) -> PrimitiveMap {
        let mut map = HashMap::new();
        for comp in &ctx.components {
            for cell in comp.cells.iter() {
                let cl: &ir::Cell = &cell.borrow();

                if let ir::CellType::Primitive {
                    name,
                    param_binding,
                } = cl.prototype.clone()
                {
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
            }
        }
        Rc::new(RefCell::new(map))
    }

    fn construct_pv_map(ctx: &ir::Context) -> PortValMap {
        let mut map = HashMap::new();
        for comp in &ctx.components {
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
                                    cll.get_parameter("VALUE")
                                        .unwrap_or_default(),
                                    pt.width,
                                )
                                .unwrap(),
                            );
                        }
                    }
                    ir::CellType::Component { .. } => {
                        for port in &cll.ports {
                            let pt: &ir::Port = &port.borrow();
                            map.insert(
                                pt as ConstPort,
                                Value::from(0, 0).unwrap(),
                            );
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        map.into()
    }

    /// Return the value associated with a component's port.
    pub fn get_from_port(&self, port: &ir::Port) -> &Value {
        &self.pv_map.get(&(port as ConstPort)).unwrap()
    }

    pub fn get_from_const_port(&self, port: *const ir::Port) -> &Value {
        &self.pv_map.get(&port).unwrap()
    }

    /// Gets the cell in a component based on the name;
    /// XXX: similar to find_cell in component.rs
    /// Does this function *need* to be in environment?
    pub fn get_cell(
        &self,
        comp: &ir::Id,
        cell: &ir::Id,
    ) -> Option<ir::RRC<ir::Cell>> {
        let a = self.context.borrow();
        let temp = a.components.iter().find(|cm| cm.name == *comp)?;
        temp.find_cell(&(cell.id))
    }

    /// Outputs the cell state;
    ///TODO (write to a specified output in the future) We could do the printing
    ///of values here for tracing purposes as discussed. Could also have a
    ///separate DS that we could put the cell states into for more custom tracing
    pub fn print_env(&self) {
        println!("{}", serde_json::to_string_pretty(&self).unwrap());
    }

    pub fn cell_is_comb(&self, cell: &ir::Cell) -> bool {
        self.cell_prim_map
            .borrow()
            .get(&(cell as ConstCell))
            .unwrap()
            .is_comb()
    }

    pub fn fork(&mut self) -> Self {
        let other_pv_map = if self.pv_map.top().is_empty() {
            self.pv_map.fork_from_tail()
        } else {
            self.pv_map.fork()
        };
        Self {
            clk: self.clk,
            cell_prim_map: Rc::clone(&self.cell_prim_map),
            pv_map: other_pv_map,
            context: Rc::clone(&self.context),
        }
    }
}

impl Serialize for InterpreterState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ctx: &ir::Context = &self.context.borrow();

        let cell_prim_map = self.cell_prim_map.borrow();

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
                    .filter_map(|cell| {
                        if let Some(prim) = cell_prim_map
                            .get(&(&cell.borrow() as &ir::Cell as ConstCell))
                        {
                            if !prim.is_comb() {
                                return Some((
                                    cell.borrow().name().clone(),
                                    prim,
                                ));
                            }
                        }
                        None
                    })
                    .collect();
                (comp.name.clone(), inner_map)
            })
            .collect();

        let p = Printable {
            ports: bmap,
            memories: cell_map,
        };
        p.serialize(serializer)
    }
}

#[derive(Serialize)]
#[allow(clippy::borrowed_box)]
struct Printable<'a> {
    ports: BTreeMap<ir::Id, BTreeMap<ir::Id, BTreeMap<ir::Id, u64>>>,
    memories: BTreeMap<ir::Id, BTreeMap<ir::Id, &'a Box<dyn Primitive>>>,
}
