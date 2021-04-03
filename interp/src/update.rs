use super::{environment::Environment, primitives};
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::collections::HashMap;

/// Stores information for updates.
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
    pub vars: HashMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct UpdateQueue {
    pub component: String,
    pub updates: Vec<Update>,
}

impl UpdateQueue {
    pub fn init(component: String) -> Self {
        Self {
            component: component,
            updates: Vec::new(),
            // let mut temp = Vec::new();
            // self.updates = temp;  }
        }
    }
    /// Initializes values for the update queue, i.e. for non-combinational cells
    #[allow(clippy::unnecessary_unwrap)]
    pub fn init_cells(
        mut self,
        cell: &ir::Id,
        inputs: Vec<ir::Id>,
        outputs: Vec<ir::Id>,
        mut env: Environment,
    ) -> Self {
        let cell_r = env
            .get_cell(&ir::Id::from(self.component), cell)
            .unwrap_or_else(|| panic!("Cannot find cell with name"));
        // get the cell type
        match cell_r.borrow().type_name() {
            None => panic!("bad"),
            Some(ct) => match ct.id.as_str() {
                "std_sqrt" => { //:(
                     // has intermediate steps/computation
                }
                "std_reg" => {
                    let map: HashMap<String, u64> = HashMap::new();
                    // reg.in = dst port should go here
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
        uvars: HashMap<String, u64>,
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
    pub fn remove_update(&mut self, ucell: &ir::Id) {
        self.updates.retain(|u| u.cell != ucell);
    }

    // TODO: should the return type be FuTIlResult<Environment>?
    /// Simulates a clock cycle by executing the stored updates.
    pub fn do_tick(
        mut self,
        environment: &Environment,
    ) -> FutilResult<Environment> {
        let mut env = environment;

        let cid = ir::Id::from(self.component.clone());

        let uq = self.updates.clone();
        // iterate through each update
        for update in uq {
            let updated = primitives::update_cell_state(
                &update.cell,
                &update.inputs,
                &update.outputs,
                environment,
                self.component.clone(),
            );
            match updated {
                Ok(updated_env) => {
                    let updated_cell = updated_env
                        .map
                        .get(&cid)
                        .unwrap_or_else(|| panic!("Can't get component's map"))
                        .get(&update.cell)
                        .clone();
                    match updated_cell {
                        Some(m) => {
                            updated_env.put_cell(&cid, *m);
                            env = &updated_env;
                        }
                        _ => panic!("Could not apply update."),
                    }
                    //updated_env.map.insert(update.cell.clone(), updated_cell);
                }
                _ => panic!("Could not apply update. "),
            }
        }
        Ok(*env)
    }
}
