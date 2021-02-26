use super::primitives;
use calyx::ir;
use std::collections::HashMap;
use std::rc::Rc;

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

/// The environment to interpret a Calyx program.
#[derive(Clone, Debug)]
pub struct Environment {
    /// A mapping from cell names to the values on their ports.
    // XXX: Should not be `pub`.
    map: HashMap<ir::Id, HashMap<ir::Id, u64>>,

    /// A queue of operations that need to be applied in the future.
    /// A vector of Updates.
    update_queue: Vec<Update>,

    // XXX(karen): Will probably need to remove eventually
    // XXX(rachit): We can probably just "attach" an `ir::Component` here and
    // use the methods defined on that (like `ir::Component::get_cell`).
    /// A mapping from cell ids to cells, much like in component.rs.
    cells: HashMap<ir::Id, ir::RRC<ir::Cell>>,
}

/// Helper functions for the environment.
impl Environment {
    /// Constructor "syntactic sugar"
    pub fn init(
        map: HashMap<ir::Id, HashMap<ir::Id, u64>>,
        cells: HashMap<ir::Id, ir::RRC<ir::Cell>>,
    ) -> Self {
        let update_queue: Vec<Update> = Vec::new();
        Self {
            map,
            update_queue,
            cells,
        }
    }

    /// Returns the value on a port, in a cell.
    // XXX(rachit): Deprecate this method in favor of `get_from_port`
    pub fn get(&self, cell: &ir::Id, port: &ir::Id) -> u64 {
        self.map[cell][port]
    }

    /// Return the value associated with an ir::Port.
    pub fn get_from_port(&self, port: &ir::Port) -> u64 {
        if port.is_hole() {
            panic!("Cannot get value from hole")
        }
        self.map[&port.get_parent_name()][&port.name]
    }

    /// Puts the mapping from cell to port to val in map.
    pub fn put(&mut self, cell: &ir::Id, port: &ir::Id, val: u64) {
        self.map
            .entry(cell.clone())
            .or_default()
            .insert(port.clone(), val);
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
        self.update_queue.push(update);
    }

    /// Convenience function to remove a particular cell's update from the update queue
    pub fn remove_update(&mut self, ucell: &ir::Id) {
        self.update_queue.retain(|u| u.cell != ucell);
    }

    // TODO: should the return type be FuTIlResult<Environment>?
    /// Simulates a clock cycle by executing the stored updates.
    pub fn do_tick(mut self) -> Self {
        let uq = self.update_queue.clone();
        for update in uq {
            let updated = primitives::update_cell_state(
                &update.cell,
                &update.inputs,
                &update.outputs,
                &self,
            );
            match updated {
                Ok(updated_env) => {
                    let updated_cell = updated_env
                        .map
                        .get(&update.cell)
                        .unwrap_or_else(|| panic!("Can't get map"))
                        .clone();
                    self.map.insert(update.cell.clone(), updated_cell);
                }
                _ => panic!("Could not apply update. "),
            }
        }
        self
    }

    /// Gets the cell based on the name;
    /// XXX: similar to find_cell in component.rs
    pub fn get_cell(&self, cell: &ir::Id) -> Option<ir::RRC<ir::Cell>> {
        self.cells
            .values()
            .find(|&g| g.borrow().name == *cell)
            .map(|r| Rc::clone(r))
    }

    /// Outputs the cell state; TODO (write to a specified output in the future)
    pub fn cell_state(&self) {
        let state_str = self
            .map
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
}
