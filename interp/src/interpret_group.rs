//! Used for the command line interface.
//! Only interprets a given group in a given component

use super::{environment::Environment, interpreter, update::Update};
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
// use std::collections::HashMap;
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
            self.component.clone(),
        )?;
        // Print out final state of environment
        finalenv.cell_state();
        Ok(finalenv)
    }
}
