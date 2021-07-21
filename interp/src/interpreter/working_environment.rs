use crate::environment::InterpreterState;
use crate::primitives::Primitive;
use crate::utils::{get_const_from_rrc, OutputValueRef};
use crate::values::{OutputValue, ReadableValue, Value};
use calyx::{
    errors::FutilResult,
    ir::{self, RRC},
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use super::utils::ConstCell;

type ConstPort = *const ir::Port;

/// A wrapper for a map assigning OutputValues to each port. Used in the working
/// environment to track values that are not of type Value which is used in the
/// environment.
// TODO (griffin): Update environment definition to allow for things of type
//                 OutputValue?
type PortOutputValMap = HashMap<ConstPort, OutputValue>;
// A wrapper struct to keep the passed environment and a map tracking the
/// updates made in the current environment. It is only really needed because
/// the environment maps to values of type Value, but during group
/// interpretation, ports need to be mapped to values of type OutputValue
// TODO (griffin): Update / remove pending changes to environment definition
pub(super) struct WorkingEnvironment {
    //InterpreterState has a pv_map which is a Smoosher<*const ir::Port, Value>
    pub backing_env: InterpreterState,
    pub working_env: PortOutputValMap, // HashMap<*const ir::Port, OutputValue>
}

impl From<InterpreterState> for WorkingEnvironment {
    fn from(input: InterpreterState) -> Self {
        Self {
            backing_env: input,
            working_env: PortOutputValMap::default(),
        }
    }
}

impl WorkingEnvironment {
    pub fn get_const(&self, port: *const ir::Port) -> OutputValueRef {
        let working_val = self.working_env.get(&port);
        match working_val {
            Some(v) => v.into(),
            None => self.backing_env.get_from_const_port(port).into(),
        }
    }
    /// Attempts to first get value from the working_env (PortOutputValMap)
    /// If doesn't exist, gets from backing_env (InterpreterState)
    pub fn get(&self, port: &ir::Port) -> OutputValueRef {
        self.get_const(port as *const ir::Port)
    }

    pub fn update_val_const_port(
        &mut self,
        port: *const ir::Port,
        value: OutputValue,
    ) {
        self.working_env.insert(port, value);
    }

    pub fn update_val(&mut self, port: &ir::Port, value: OutputValue) {
        self.update_val_const_port(port as *const ir::Port, value);
    }

    pub fn get_as_val_const(&self, port: *const ir::Port) -> &Value {
        match self.get_const(port) {
            OutputValueRef::ImmediateValue(iv) => iv.get_val(),
            OutputValueRef::LockedValue(tlv) => tlv.get_val(),
            OutputValueRef::PulseValue(pv) => pv.get_val(),
        }
    }

    pub fn get_as_val(&self, port: &ir::Port) -> &Value {
        self.get_as_val_const(port as *const ir::Port)
    }

    //for use w/ smoosher: maybe add a new scope onto backing_env for the tick?
    pub fn do_tick(&mut self) {
        self.backing_env.clk += 1;

        let mut w_env = std::mem::take(&mut self.working_env);

        self.working_env = w_env
            .drain()
            .filter_map(|(port, val)| match val {
                OutputValue::ImmediateValue(iv) => {
                    self.backing_env.insert(port, iv); //if you have an IV, remove from WorkingEnv and put in BackingEnv
                    None
                }
                out @ OutputValue::PulseValue(_)
                | out @ OutputValue::LockedValue(_) => match out.do_tick() {
                    OutputValue::ImmediateValue(iv) => {
                        self.backing_env.insert(port, iv); //if you have a Locked/PulseValue, tick it, and if it's now IV, put in BackEnv
                        None
                    }
                    v @ OutputValue::LockedValue(_) => Some((port, v)),
                    OutputValue::PulseValue(pv) => Some((port, pv.into())),
                },
            })
            .collect();
    }

    pub fn collapse_env(
        mut self,
        panic_on_invalid_val: bool,
    ) -> InterpreterState {
        let working_env = self.working_env;

        for (port, v) in working_env {
            match v {
                OutputValue::ImmediateValue(iv) => {
                    self.backing_env.insert(port, iv)
                }
                OutputValue::LockedValue(tlv) => {
                    if tlv.unlockable() {
                        let iv = tlv.unlock();
                        self.backing_env.insert(port, iv);
                    } else if panic_on_invalid_val {
                        panic!("Group is done with an invalid value?")
                    } else if let Some(old) = tlv.old_value {
                        self.backing_env.insert(port, old)
                    }
                }
                OutputValue::PulseValue(v) => {
                    self.backing_env.insert(port, v.take_val())
                }
            }
        }
        self.backing_env
    }

    // // For debugging purpose
    // pub fn _dump_state(&self, cell: &ir::Cell) {
    //     println!("{} on cycle {}: ", cell.name(), self.backing_env.clk);
    //     for p in &cell.ports {
    //         let p_ref: &ir::Port = &p.borrow();
    //         println!("  {} : {}", p_ref.name, self.get_as_val(p_ref).as_u64());
    //     }
    //     match self
    //         .backing_env
    //         .cell_prim_map
    //         .borrow()
    //         .get(&(cell as *const ir::Cell))
    //         .unwrap()
    //     {
    //         Primitive::StdReg(ref reg) => {
    //             println!("  internal state: {}", reg.val)
    //         }
    //         Primitive::StdMemD1(ref mem) => {
    //             println!("  memval : {}", mem.data[0])
    //         }
    //         _ => {}
    //     }
    // }

    pub fn eval_guard(&self, guard: &ir::Guard) -> bool {
        match guard {
            ir::Guard::Or(g1, g2) => self.eval_guard(g1) || self.eval_guard(g2),
            ir::Guard::And(g1, g2) => {
                self.eval_guard(g1) && self.eval_guard(g2)
            }
            ir::Guard::Not(g) => !self.eval_guard(g),
            ir::Guard::Eq(g1, g2) => {
                self.get_as_val(&g1.borrow()) == self.get_as_val(&g2.borrow())
            }
            ir::Guard::Neq(g1, g2) => {
                self.get_as_val(&g1.borrow()) != self.get_as_val(&g2.borrow())
            }
            ir::Guard::Gt(g1, g2) => {
                self.get_as_val(&g1.borrow()) > self.get_as_val(&g2.borrow())
            }
            ir::Guard::Lt(g1, g2) => {
                self.get_as_val(&g1.borrow()) < self.get_as_val(&g2.borrow())
            }
            ir::Guard::Geq(g1, g2) => {
                self.get_as_val(&g1.borrow()) >= self.get_as_val(&g2.borrow())
            }
            ir::Guard::Leq(g1, g2) => {
                self.get_as_val(&g1.borrow()) <= self.get_as_val(&g2.borrow())
            }
            ir::Guard::Port(p) => {
                let val = self.get_as_val(&p.borrow());
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

    fn get_inputs<'a>(&'a self, cell: &ir::Cell) -> Vec<(ir::Id, &'a Value)> {
        cell.ports
            .iter()
            .filter_map(|p| {
                let p_ref: &ir::Port = &p.borrow();
                match &p_ref.direction {
                    ir::Direction::Input => {
                        Some((p_ref.name.clone(), self.get_as_val(p_ref)))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Evaluates the primitives corresponding to the given iterator of cells, based
    /// on the current environment. Returns a set of assignments that may change
    /// based on the updates to primitive values.
    ///
    /// Note: this function could be written with only one lifetime, but it is worth
    /// noting that the returned assignments refs are tied to the dependency map and
    /// thus to the assignments it is referencing meanwhile the lifetime on the
    /// given cell RRCs is unrelated and largely irrelevant as the prim_map is keyed
    /// off of port raw pointers whose lifetime is uncoupled from the cells.
    pub fn eval_prims<'a, 'b, I: Iterator<Item = &'b RRC<ir::Cell>>>(
        &mut self,
        exec_list: I,
        reset_flag: bool, // reset vals or execute normally
    ) -> bool {
        let mut val_changed = false;
        // split mutability
        // TODO: change approach based on new env, once ready
        let ref_clone = self.backing_env.cell_prim_map.clone(); // RC clone
        let mut prim_map = ref_clone.borrow_mut();

        let mut update_list: Vec<(RRC<ir::Port>, OutputValue)> = vec![];

        for cell in exec_list {
            let inputs = self.get_inputs(&cell.borrow());

            let executable = prim_map.get_mut(&get_const_from_rrc(&cell));

            if let Some(prim) = executable {
                let new_vals = if reset_flag {
                    prim.clear_update_buffer();
                    prim.reset(&inputs)
                } else {
                    let done_val =
                        if prim.is_comb() {
                            None
                        } else {
                            Some(self.get_as_val(
                                &(cell.borrow().get("done").borrow()),
                            ))
                        };
                    prim.validate_and_execute(&inputs, done_val)
                };

                for (port, val) in new_vals {
                    let port_ref = cell.borrow().find(port).unwrap();

                    let current_val = self.get(&port_ref.borrow());

                    if current_val != (&val).into() {
                        val_changed = true;
                        // defer value update until after all executions
                        update_list.push((Rc::clone(&port_ref), val));
                    }
                }
            }
        }

        for (port, val) in update_list {
            self.update_val(&port.borrow(), val);
        }

        val_changed
    }

    pub fn state_as_str(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
}

// This is basically a copy-paste from InterpreterState
impl Serialize for WorkingEnvironment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ctx: &ir::Context = &self.backing_env.context.borrow();

        let cell_prim_map = self.backing_env.cell_prim_map.borrow();

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
                                    self.get_as_val(&port.borrow()).as_u64(),
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
