//! Used for the command line interface.
//! Only interprets a given group in a given component

use super::interpreter;
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::HashMap;
use std::rc::Rc;

/// Stores information about the component and group to interpret.
/// Might be better to make this a subset of a trait implemented by all interpreters, later on
pub struct GroupInterpreter {
    /// The name of the component with the group to interpret
    pub component: String,
    /// The name of the group to interpret
    pub group: String,
}

impl GroupInterpreter {
    /// Returns the name of the interpreter
    pub fn name(self) -> &'static str {
        "group interpreter"
    }

    /// Interpret a group, given a context, component name, and group name
    pub fn interpret(self, ctx: ir::Context) -> FutilResult<()> {
        // Validation
        let comp = get_component(ctx, &self.component)?;

        // Intialize environment
        let map = construct_map(&comp.cells);
        let cellmap = comp
            .cells
            .iter()
            .map(|cell| (cell.borrow().name.clone(), Rc::clone(&cell)))
            .collect::<HashMap<_, _>>();

        // Initial state of the environment
        let environment = interpreter::Environment::init(map, cellmap);
        environment.cell_state();

        // Interpret the group
        let group = comp.find_group(&self.group).ok_or_else(|| {
            Error::Undefined(ir::Id::from(self.group), "group".to_string())
        })?;

        // Final state of the environment
        let finalenv = interpreter::eval_group(group, environment)?;
        finalenv.cell_state();
        Ok(())
    }
}

// Get the name of the component to interpret from the context.
fn get_component<'a>(
    ctx: ir::Context,
    component: &str,
) -> FutilResult<ir::Component> {
    match ctx.components.into_iter().find(|c| c.name.id == *component) {
        Some(comp) => Ok(comp),
        None => Err(Error::Undefined(
            ir::Id::from(component.to_string()),
            "component".to_string(),
        )),
    }
}

// Construct a map from cell ids to a map from the cell's ports' ids to the ports' values
fn construct_map(
    cells: &[ir::RRC<ir::Cell>],
) -> HashMap<ir::Id, HashMap<ir::Id, u64>> {
    let mut map = HashMap::new();
    for cell in cells {
        let cb = cell.borrow();
        let mut ports: HashMap<ir::Id, u64> = HashMap::new();

        match &cb.prototype {
            // A FuTIL constant cell's out port is that constant's value
            ir::CellType::Constant { val, .. } => {
                ports.insert(ir::Id::from("out"), *val);
                map.insert(cb.name.clone(), ports);
            }
            ir::CellType::Primitive { .. } => {
                for port in &cb.ports {
                    // All ports for primitives are initalized to 0 , unless the cell is an std_const
                    let pb = port.borrow();
                    let initval = cb
                        .get_paramter(&ir::Id::from("value".to_string()))
                        .unwrap_or(0); //std_const should be the only cell type with the "value" parameter

                    ports.insert(pb.name.clone(), initval);
                }
                map.insert(cb.name.clone(), ports);
            }
            _ => panic!("component"),
        }
    }
    map
}
