//! Environment for interpreter.

use super::{primitives, values::Value};
use calyx::{errors::FutilResult, ir, ir::CloneName};
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryInto;
//use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Update {
    /// The cell to be updated
    pub cell: ir::Id,
    /// The vector of input ports
    pub inputs: Vec<ir::Id>,
    /// The vector of output ports
    pub outputs: Vec<ir::Id>,
    /// Map of intermediate variables
    /// (could refer to a port or it could be "new", e.g. in the sqrt)
    pub vars: HashMap<ir::Id, u64>,
}

/// Queue of updates.
#[derive(Clone, Debug)]
pub struct UpdateQueue {
    pub component: ir::Id,
    pub updates: Vec<Update>,
}

impl UpdateQueue {
    // TODO: incomplete
    pub fn init(component: ir::Id) -> Self {
        Self {
            component,
            updates: Vec::new(),
            // let mut temp = Vec::new();
            // self.updates = temp;  }
        }
    }
    /// Initializes values for the update queue, i.e. for non-combinational cells
    /// inputs : Vector of input...
    /// outputs : Vector of output...
    /// env : Environment
    #[allow(clippy::unnecessary_unwrap)]
    pub fn init_cells(
        mut self,
        cell: &ir::Id,
        inputs: Vec<ir::Id>,
        outputs: Vec<ir::Id>,
        env: Environment,
    ) -> Self {
        let cell_r = env
            .get_cell(&self.component, cell)
            .unwrap_or_else(|| panic!("Cannot find cell with name"));
        // get the cell type
        match cell_r.borrow().type_name() {
            None => panic!("bad"),
            Some(ct) => match ct.id.as_str() {
                "std_sqrt" => { //:(
                     // has intermediate steps/computation
                }
                "std_reg" => {
                    let map: HashMap<ir::Id, u64> = HashMap::new();
                    // reg.in = dst port should go here
                    self.add_update(cell.clone(), inputs, outputs, map);
                }
                "std_mem_d1" => {
                    let map: HashMap<ir::Id, u64> = HashMap::new();
                    self.add_update(cell.clone(), inputs, outputs, map);
                }
                _ => panic!(
                    "attempted to initalize an update for a combinational cell"
                ),
            },
        }
        self
    }

    /// Adds an update to the update queue; TODO; ok to drop prev and next?
    pub fn add_update(
        &mut self,
        ucell: ir::Id,
        uinput: Vec<ir::Id>,
        uoutput: Vec<ir::Id>,
        uvars: HashMap<ir::Id, u64>,
    ) {
        //println!("add update!");
        let update = Update {
            cell: ucell,
            inputs: uinput,
            outputs: uoutput,
            vars: uvars,
        };
        self.updates.push(update);
    }

    /// Convenience function to remove a particular cell's update from the update queue
    /// TODO: what if I have reg0.in = (4) and reg0.in = (5) in the program?
    pub fn _remove_update(&mut self, ucell: &ir::Id) {
        self.updates.retain(|u| u.cell != ucell);
    }

    /// Simulates a clock cycle by executing the stored updates.
    pub fn do_tick(self, environment: Environment) -> FutilResult<Environment> {
        let mut env = environment;
        let uq = self.updates.clone();
        // iterate through each update
        for update in uq {
            let updated = primitives::update_cell_state(
                &update.cell,
                &update.inputs,
                &update.outputs,
                &(env.clone()),
                self.component.clone(),
            )?;
            env = updated.clone();
        }
        Ok(env)
    }
}

/// The environment to interpret a Calyx program.
#[derive(Clone, Debug)]
pub struct Environment {
    /// Stores values of context.
    /// Maps component names to a mapping from the component's cell names to their ports' values.
    pub map: HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, Value>>>,

    pub clk: u64,
    ///mapping from cells to prims
    ///clock count
    ///use raw pointers for hashmap: ports to values
    pub pv_map: HashMap<*const ir::Port, Value>,

    /// A reference to the context.
    pub context: ir::RRC<ir::Context>,
}

/// Helper functions for the environment.
impl Environment {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init(ctx: &ir::RRC<ir::Context>) -> Self {
        Self {
            map: Environment::construct_map(&ctx.borrow()),
            context: ctx.clone(),
            clk: 0,
            pv_map: Environment::construct_pv_map(),
        }
    }

    fn construct_pv_map() -> HashMap<*const ir::Port, Value> {
        todo!();
    }

    /// Returns the value on a port, in a component's cell.
    // XXX(rachit): Deprecate this method in favor of `get_from_port`
    pub fn get(
        &self,
        component: &ir::Id,
        cell: &ir::Id,
        port: &ir::Id,
    ) -> Value {
        self.map[component][cell][port]
    }

    /// Return the value associated with a component's port.
    pub fn get_from_port(&self, component: &ir::Id, port: &ir::Port) -> Value {
        if port.is_hole() {
            panic!("Cannot get value from hole")
        }
        self.map[component][&port.get_parent_name()][&port.name]
    }

    /// Puts a mapping from component to cell to port to val into map.
    pub fn put(
        &mut self,
        comp: &ir::Id,
        cell: &ir::Id,
        port: &ir::Id,
        val: Value,
    ) {
        self.map
            .entry(comp.clone())
            .or_default()
            .entry(cell.clone())
            .or_default()
            .insert(port.clone(), val);
    }

    /// Puts a mapping from component to cell to port to val into map.
    /// Should this function return the modified environment instead?
    pub fn _put_cell(
        &mut self,
        comp: &ir::Id,
        cellport: HashMap<ir::Id, Value>,
    ) {
        self.map
            .entry(comp.clone())
            .or_default()
            .insert(comp.clone(), cellport);
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

    /// Maps components to maps from cell ids to a map from the cell's ports' ids to port values
    fn construct_map(
        context: &ir::Context,
    ) -> HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, Value>>> {
        let mut map = HashMap::new();
        for comp in &context.components {
            let mut cell_map = HashMap::new();
            for cell in comp.cells.iter() {
                let cb = cell.borrow();
                let mut ports: HashMap<ir::Id, Value> = HashMap::new();
                match &cb.prototype {
                    // A FuTIL constant cell's out port is that constant's value
                    ir::CellType::Constant { val, width } => {
                        ports.insert(
                            ir::Id::from("out"),
                            Value::from_init::<usize>(
                                (*val as usize).try_into().unwrap(),
                                (*width as usize).try_into().unwrap(),
                            ),
                        );
                        cell_map.insert(cb.clone_name(), ports);
                    }
                    ir::CellType::Primitive { .. } => {
                        for port in &cb.ports {
                            // All ports for primitives are initalized to 0 , unless the cell is an std_const
                            let pb = port.borrow();
                            let initval = cb
                                .get_paramter(&ir::Id::from(
                                    "value".to_string(),
                                ))
                                .unwrap_or(0); //std_const should be the only cell type with the "value" parameter
                            ports.insert(
                                pb.name.clone(),
                                Value::from_init::<usize>(
                                    (initval as usize).try_into().unwrap(),
                                    (pb.width as usize).try_into().unwrap(),
                                ),
                            );
                        }
                        cell_map.insert(cb.clone_name(), ports);
                    }
                    //TODO: handle components
                    _ => panic!("component"),
                }
            }
            map.insert(comp.name.clone(), cell_map);
        }
        map
    }

    /// Outputs the cell state;
    ///TODO (write to a specified output in the future) We could do the printing
    ///of values here for tracing purposes as discussed. Could also have a
    ///separate DS that we could put the cell states into for more custom tracing
    pub fn print_env(&self) {
        println!("{}", serde_json::to_string_pretty(&self).unwrap());
    }
}

impl Serialize for Environment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // use collect to make the nested hashmap a nested btreemap
        let ordered: BTreeMap<_, _> = self
            .map
            .iter()
            .map(|(id, map)| {
                let inner_map: BTreeMap<_, _> = map
                    .iter()
                    .map(|(id, map)| {
                        let inner_map: BTreeMap<_, _> = map
                            .iter()
                            .map(|(id, val)| (id, val.as_u64()))
                            .collect();
                        (id, inner_map)
                    })
                    .collect();
                (id, inner_map)
            })
            .collect();
        ordered.serialize(serializer)
    }
}
