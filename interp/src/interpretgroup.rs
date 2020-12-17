//! Used for the command line interface
//! Only interprets a given group in a given component

use super::interpreter;
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::HashMap;
use std::rc::Rc;

// might be better to make this part of a trait implemented by all interpreters, later on
pub struct GroupInterpreter {
    // the name of the component
    pub component: String,
    // the name of the group to interpret
    pub group: String,
}

impl GroupInterpreter {
    // Returns the name of the interpreter
    pub fn name(self) -> &'static str {
        "group interpreter"
    }

    // Interpret a group, given a context, component name, and group name
    pub fn interpret(self, ctx: ir::Context) -> FutilResult<()> {
        // validation
        let comp = validate_names(&ctx, &self.component, &self.group)?;

        // intialize environment
        let cells = get_cells(&ctx, &self.component); // May not necessarily need to explicitly get cells here?
        let map = construct_map(&cells);
        let cellmap = construct_cell_map(&cells);

        //println!("cells and ports: {:?}", map);
        //println!("ids and cells: {:?}", cellmap);

        let environment: interpreter::Environment =
            interpreter::Environment::init(map, cellmap);

        // Initial state of the environment
        environment.cell_state();

        // interpret the group
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
// Lifetime??
fn validate_names<'a>(
    ctx: &'a ir::Context,
    component: &String,
    group: &String,
) -> FutilResult<&'a ir::Component> {
    let components = &ctx.components;

    match components.into_iter().find(|&c| c.name.id == *component) {
        Some(comp) => {
            //let g = comp.find_group(group);
            let groups = &comp.groups.clone();
            match groups.into_iter().find(|&g| g.borrow().name == *group) {
                Some(_) => Ok(comp),
                None => Err(Error::UndefinedGroup(ir::Id::from(group.clone()))),
            }
        }
        None => Err(Error::UndefinedComponent(ir::Id::from(component.clone()))),
    }
}

// Find the component's cells in context; duplicated code?
fn get_cells(ctx: &ir::Context, component: &String) -> Vec<ir::RRC<ir::Cell>> {
    let components = &ctx.clone().components;
    match components.into_iter().find(|&c| c.name.id == *component) {
        Some(comp) => comp.cells.clone(),
        _ => panic!("This isn't supposed to happen?"),
    }
}

// Construct a map from cell ids to a map to the cell's port's ids to the port values
fn construct_map(
    cells: &Vec<ir::RRC<ir::Cell>>,
) -> HashMap<ir::Id, HashMap<ir::Id, u64>> {
    let mut map = HashMap::new();
    for cell in cells {
        let cb = cell.borrow();
        let mut ports: HashMap<ir::Id, u64> = HashMap::new();

        match &cb.prototype {
            // constant cell's out port is the constant's value
            ir::CellType::Constant { val, .. } => {
                ports.insert(ir::Id::from("out"), *val);
                map.insert(cb.name.clone(), ports);
            }
            ir::CellType::Primitive { .. } => {
                for port in &cb.ports {
                    // all ports initalized to 0 for now, unless the cell is an std_constant (or the port is write_en)
                    let pb = port.borrow();
                    if pb.name == "write_en" {
                        let initval = 0; //TODO: write_en is de facto 1 for now

                        ports.insert(pb.name.clone(), initval);
                    } else {
                        let initval = cb
                            .get_paramter(&ir::Id::from("value".to_string()))
                            .unwrap_or_else(|| 0); //should be that only std_const has "value" parameter

                        ports.insert(pb.name.clone(), initval);
                    }
                }
                map.insert(cb.name.clone(), ports);
            }
            _ => panic!("component"),
        }
    }
    map
}

// Construct a map from cell ids to cells; may be temporary
fn construct_cell_map(
    cells: &Vec<ir::RRC<ir::Cell>>,
) -> HashMap<ir::Id, ir::RRC<ir::Cell>> {
    let mut map = HashMap::new();
    for cell in cells {
        map.insert(cell.borrow().name.clone(), Rc::clone(&cell));
    }
    map
}
