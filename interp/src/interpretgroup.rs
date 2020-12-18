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
        let comp = validate_names(&ctx, &self.component, &self.group)?;

        // Intialize environment
        let cells = get_cells(&ctx, &self.component); // May not necessarily need to explicitly get cells here?
        let map = construct_map(&cells);
        let cellmap = construct_cell_map(&cells);

        let environment: interpreter::Environment =
            interpreter::Environment::init(map, cellmap);

        // Initial state of the environment
        environment.cell_state();

        // Interpret the group
        let group = comp
            .find_group(&self.group)
            .unwrap_or_else(|| panic!("bad"));
        //println!("Started interpreting...");

        let finalenv = interpreter::eval_group(group, environment)?;

        //println!("Finished interpreting.");
        // Final state of the environment
        finalenv.cell_state();
        Ok(())
    }
}

// Ensures that the component and group names exist in the context
fn validate_names<'a>(
    ctx: &'a ir::Context,
    component: &str,
    group: &str,
) -> FutilResult<&'a ir::Component> {
    let components = &ctx.components;

    match components.iter().find(|&c| c.name.id == *component) {
        Some(comp) => {
            let groups = &comp.groups.clone();
            match groups.iter().find(|&g| g.borrow().name == *group) {
                Some(_) => Ok(comp),
                None => Err(Error::Undefined(
                    ir::Id::from(group.to_string()),
                    "group".to_string(),
                )),
            }
        }
        None => Err(Error::Undefined(
            ir::Id::from(component.to_string()),
            "component".to_string(),
        )),
    }
}

// Find the component's cells in context; duplicated code?
fn get_cells(ctx: &ir::Context, component: &str) -> Vec<ir::RRC<ir::Cell>> {
    let components = &ctx.components;
    match components.iter().find(|&c| c.name.id == *component) {
        Some(comp) => comp.cells.clone(),
        _ => panic!("This isn't supposed to happen?"),
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

// Construct a map from cell ids to cells; this is likely necessary for a group interpreter.
// However, it is probably unecessary for a component interpreter.
fn construct_cell_map(
    cells: &[ir::RRC<ir::Cell>],
) -> HashMap<ir::Id, ir::RRC<ir::Cell>> {
    let mut map = HashMap::new();
    for cell in cells {
        map.insert(cell.borrow().name.clone(), Rc::clone(&cell));
    }
    map
}
