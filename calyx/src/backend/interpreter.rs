use crate::errors::FutilResult;
use crate::ir;
use std::collections::HashMap;

/// The environment to interpret a FuTIL program
#[derive(Default, Clone)]
pub struct Environment {
    /// A mapping from cell names to the values on their ports.
    map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
}

/// Helper functions for the environment.
impl Environment {
    /// Returns the value on a port, in a cell.
    pub fn get(&self, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[cell][port]
    }
}

pub fn eval(comp: &ir::Component) -> FutilResult<Environment> {
    todo!()
}

pub fn eval_assigns(assigns: &[ir::Assignment]) -> FutilResult<Environment> {
    todo!()
}
