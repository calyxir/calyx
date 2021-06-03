//! Used for the command line interface.
//! Only interprets a given group in a given component

use super::{environment::Environment, interpreter /*update::Update */};
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::HashMap;
// use std::rc::Rc;

/// Stores information about the component and group to interpret.
/// Might be better to make this a subset of a trait implemented by all interpreters, later on
pub struct GroupInterpreter {
    /// The name of the component with the group to interpret
    pub component: String,
    /// The group to interpret
    pub group: ir::RRC<ir::Group>,
    /// The environment for the interpreter.
    pub environment: Environment,
}

impl GroupInterpreter {
    /// Construct a GroupInterpreter
    /// comp: Name of component the group is from
    /// grp: The group to interpret
    /// env: The initial environment
    pub fn init(
        comp: String,
        grp: ir::RRC<ir::Group>,
        env: Environment,
    ) -> Self {
        Self {
            component: comp,
            group: grp,
            environment: env,
        }
    }

    /// Interpret this group
    pub fn interpret(self) -> FutilResult<Environment> {
        // Print the initial state of the environment
        // self.environment.cell_state(self.component.clone());

        // Final state of the environment
        let finalenv = interpreter::eval_group(
            self.group,
            self.environment,
            self.component,
        )?;
        // Print out final state of environment
        finalenv.print_env();
        Ok(finalenv)
    }
}

/// Get the name of the component to interpret from the context.
fn _get_component(
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

/// Construct a map from cell ids to a map from the cell's ports' ids to the ports' values
fn _construct_map(
    cells: &[ir::RRC<ir::Cell>],
) -> HashMap<ir::Id, HashMap<ir::Id, u64>> {
    let mut map = HashMap::new();
    for cell in cells {
        let cb = cell.borrow();
        let mut ports: HashMap<ir::Id, u64> = HashMap::new();

        match &cb.prototype {
            // A Calyx constant cell's out port is that constant's value
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
