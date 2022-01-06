use crate::ir::{self, CloneName, RRC};
use itertools::Itertools;
use std::{iter, rc::Rc};

/// Calcuate the reads-from and writes-to set for a given set of assignments.
pub struct ReadWriteSet;

impl ReadWriteSet {
    /// Returns [ir::Port] that are read from in the given Assignment.
    pub fn port_reads(
        assign: &ir::Assignment,
    ) -> impl Iterator<Item = RRC<ir::Port>> + '_ {
        assign
            .guard
            .all_ports()
            .into_iter()
            .chain(iter::once(Rc::clone(&assign.src)))
            .filter(|port| !port.borrow().is_hole())
    }

    /// Returns [ir::Port] which are read from in the assignments.
    pub fn port_read_set(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Port>> + '_ {
        assigns.iter().flat_map(Self::port_reads)
    }

    /// Returns [ir::Port] which are written to in the assignments.
    pub fn port_write_set(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Port>> + '_ {
        assigns
            .iter()
            .map(|assign| Rc::clone(&assign.dst))
            .filter(|port| !port.borrow().is_hole())
    }

    /// Returns [ir::Cell] which are read from in the assignments.
    /// **Ignores** reads from group holes.
    pub fn read_set(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Cell>> + '_ {
        Self::port_read_set(assigns)
            .map(|port| Rc::clone(&port.borrow().cell_parent()))
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns [ir::Cell] which are written to by the assignments.
    /// **Ignores** reads from group holes.
    pub fn write_set(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Cell>> + '_ {
        Self::port_write_set(assigns)
            .map(|port| Rc::clone(&port.borrow().cell_parent()))
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns the register cells whose out port is read anywhere in the given
    /// assignments
    pub fn register_reads(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Cell>> + '_ {
        fn is_register_out(port_ref: RRC<ir::Port>) -> Option<RRC<ir::Cell>> {
            let port = port_ref.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                if &port.name == "out" {
                    return Some(Rc::clone(&cell_wref.upgrade()));
                }
            }
            None
        }
        let guard_ports = assigns.iter().flat_map(|assign| {
            assign
                .guard
                .all_ports()
                .into_iter()
                .filter_map(is_register_out)
        });
        assigns
            .iter()
            .filter_map(|assign| is_register_out(Rc::clone(&assign.src)))
            .chain(guard_ports)
            .filter(|x| {
                if let Some(name) = x.borrow().type_name() {
                    name == "std_reg"
                } else {
                    false
                }
            })
            .unique_by(|cell| cell.clone_name())
    }

    /// Return the name of the cells that these assignments write to for writes
    /// that are guarded by true.
    /// **Ignores** writes to group holes.
    pub fn must_write_set(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Cell>> + '_ {
        assigns
            .iter()
            .filter_map(|assignment| {
                if let ir::Guard::True = *assignment.guard {
                    let dst_ref = assignment.dst.borrow();
                    if let ir::PortParent::Cell(cell_wref) = &dst_ref.parent {
                        return Some(Rc::clone(&cell_wref.upgrade()));
                    }
                }
                None
            })
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns all uses of cells in this group. Uses constitute both reads and
    /// writes to cells.
    pub fn uses(
        assigns: &[ir::Assignment],
    ) -> impl Iterator<Item = RRC<ir::Cell>> + '_ {
        let reads = Self::read_set(assigns);
        reads
            .chain(Self::write_set(assigns))
            .unique_by(|cell| cell.clone_name())
    }
}
