use crate::errors::FutilResult;
use crate::ir;
use std::collections::HashMap;

/// The environment to interpret a FuTIL program
#[derive(Default, Clone)]
pub struct Environment {
    /// A mapping from cell names to the values on their ports.
    map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
    /// A queue of operations that need to be applied in the future.
    update_queue: Vec<HashMap<ir::Id, HashMap<ir::Id, u64>>>,
}

/// Helper functions for the environment.
impl Environment {
    /// Returns the value on a port, in a cell.
    pub fn get(&self, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[cell][port]
    }

    /// Performs an update to the current environment using the update_queue.
    pub fn do_tick(self) -> Self {
        todo!()
    }
}

pub fn eval(comp: &ir::Component) -> FutilResult<Environment> {
    todo!()
}

fn eval_assigns(
    assigns: &[ir::Assignment],
    env: Environment,
) -> FutilResult<Environment> {
    // Find the done signal in the sequence of assignments

    // while done signal is zero
        // e2 = Clone the current environment
        // for assign in assigns
            // check if the assign.guard == 1
            // perform a read from `env` for assign.src
            // write to assign.dst to e2
            // update internal state of the cell and
            // queue any required updates.
        // env = env.do_tick()

    // Ok(env)

    todo!()
}

/// Returns the done signal in this sequence of assignments
fn get_done_signal(assigns: &[ir::Assignment]) -> &ir::Assignment {
    todo!()
}

/// Uses the cell's inputs ports to perform any required updates to the
/// cell's output ports.
fn update_cell_state(
    cell: &ir::Id,
    inputs: &[ir::Id],
    output: &[ir::Id],
    env:Environment
) -> FutilResult<Environment> {
    todo!()
}
