use std::collections::HashSet;

use crate::analysis::ControlPorts;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
};
use crate::ir::{CloneName, RRC};

/// Removes assignments to combinational elements that are never read.
#[derive(Default)]
struct DeadAssignRemoval {
    all_reads: HashSet<ir::Id>,
}

impl Named for DeadAssignRemoval {
    fn name() -> &'static str {
        "dead-assign-removal"
    }

    fn description() -> &'static str {
        "removes assignments to combinational primitives that are never read"
    }
}

// Return a port's parent cell name if it is an output port on a combinational
// cell.
fn get_port(port: &RRC<ir::Port>) -> Option<ir::Id> {
    let port = port.borrow();
    if port.direction == ir::Direction::Output {
        let cr = port.cell_parent();
        let cell = cr.borrow();
        match &cell.prototype {
            ir::CellType::Primitive { is_comb, .. } if *is_comb => {
                return Some(cell.clone_name());
            }
            _ => (),
        };
    }
    None
}

impl Visitor for DeadAssignRemoval {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &ir::LibrarySignatures,
        _comps: &[ir::Component],
    ) -> VisResult {
        // Add all combinational cells that have at least one output read.
        comp.for_each_assignment(|assign| {
            assign.for_each_port(|port| {
                if let Some(port) = get_port(port) {
                    self.all_reads.insert(port);
                }
                None
            });
        });

        // Add all of the ports in the control program.
        ControlPorts::<false>::from(&*comp.control.borrow())
            .get_ports()
            .for_each(|port| {
                if let Some(port) = get_port(&port) {
                    self.all_reads.insert(port);
                }
            });

        Ok(Action::Stop)
    }
}
