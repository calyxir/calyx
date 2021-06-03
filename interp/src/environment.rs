//! Environment for interpreter.

//use super::{primitives, update};
use calyx::ir;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
//use std::rc::Rc;

// #[derive(Serialize, Debug)]
// struct Cycle (HashMap<String, HashMap<String, u64>>);

/// The environment to interpret a Calyx program.
#[derive(Clone, Debug)]
pub struct Environment {
    /// Stores values of context.
    /// Maps component names to a mapping from the component's cell names to their ports' values.
    pub map: HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, u64>>>,

    /// A reference to the context.
    pub context: ir::RRC<ir::Context>,
}

/// Helper functions for the environment.
impl Environment {
    /// Construct an environment
    /// ctx : A context from the IR
    pub fn init(ctx: ir::RRC<ir::Context>) -> Self {
        Self {
            map: Environment::construct_map(&ctx.borrow()),
            context: ctx.clone(),
        }
    }

    /// Returns the value on a port, in a component's cell.
    // XXX(rachit): Deprecate this method in favor of `get_from_port`
    pub fn get(&self, component: &ir::Id, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[component][cell][port]
    }

    /// Return the value associated with a component's port.
    pub fn get_from_port(&self, component: &ir::Id, port: &ir::Port) -> u64 {
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
        val: u64,
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
    pub fn _put_cell(&mut self, comp: &ir::Id, cellport: HashMap<ir::Id, u64>) {
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
    ) -> HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, u64>>> {
        let mut map = HashMap::new();
        for comp in &context.components {
            let mut cell_map = HashMap::new();
            for cell in &comp.cells {
                let cb = cell.borrow();
                let mut ports: HashMap<ir::Id, u64> = HashMap::new();
                match &cb.prototype {
                    // A FuTIL constant cell's out port is that constant's value
                    ir::CellType::Constant { val, .. } => {
                        ports.insert(ir::Id::from("out"), *val);
                        cell_map.insert(cb.name.clone(), ports);
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
                            ports.insert(pb.name.clone(), initval);
                        }
                        cell_map.insert(cb.name.clone(), ports);
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
                        let inner_map: BTreeMap<_, _> = map.iter().collect();
                        (id, inner_map)
                    })
                    .collect();
                (id, inner_map)
            })
            .collect();
        ordered.serialize(serializer)
    }
}
