use super::{environment::Environment, primitives};
use calyx::{errors::FutilResult, ir};
use std::collections::HashMap;

/// Stores information for individual updates.
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
