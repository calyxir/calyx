//use super::{primitives, update};
use calyx::ir;
use std::collections::HashMap;
use std::rc::Rc;

/// The environment to interpret a FuTIL program.
#[derive(Debug)]
pub struct Environment {
    /// Maps component names to a mapping from the component's cell names to their ports' values.
    pub map: HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, u64>>>,

    /// The context.
    pub context: ir::Context,
    // The vector of components.
    //pub components: Vec<ir::Component>,
}

/// Helper functions for the environment.
impl Environment {
    /// Construct an environment from a context
    pub fn init(context: ir::Context) -> Self {
        // TODO; moving context ok?
        Self {
            map: Environment::construct_map(&context),
            context: context,
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
    pub fn put_cell(&mut self, comp: &ir::Id, cellport: HashMap<ir::Id, u64>) {
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
        let temp =
            self.context.components.iter().find(|cm| cm.name == *comp)?;
        temp.find_cell(&(cell.id))
    }

    /// Outputs a component's cell state; TODO (write to a specified output in the future)
    pub fn cell_state(&self, comp: String) {
        // TODO

        let temp = ir::Id::from(comp);

        let state_str = self.map[&temp]
            .iter()
            .map(|(cell, ports)| {
                format!(
                    "{}\n{}",
                    cell,
                    ports
                        .iter()
                        .map(|(p, v)| format!("\t{}: {}", p, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        println!("{}\n{}\n{}", "=".repeat(30), state_str, "=".repeat(30))
    }

    /// Maps components to maps from cell ids to a map from the cell's ports' ids to port values
    fn construct_map(
        context: &ir::Context,
    ) -> HashMap<ir::Id, HashMap<ir::Id, HashMap<ir::Id, u64>>> {
        let mut map = HashMap::new();
        for comp in &context.components {
            for cell in &comp.cells {
                let mut cellMap = HashMap::new();
                let cb = cell.borrow();
                let mut ports: HashMap<ir::Id, u64> = HashMap::new();
                match &cb.prototype {
                    // A FuTIL constant cell's out port is that constant's value
                    ir::CellType::Constant { val, .. } => {
                        ports.insert(ir::Id::from("out"), *val);
                        cellMap.insert(cb.name.clone(), ports);
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
                        cellMap.insert(cb.name.clone(), ports);
                    }
                    _ => panic!("component"),
                }
                map.insert(comp.name.clone(), cellMap);
            }
        }
        map
    }
}
