use crate::ir::{self, CloneName};
use itertools::Itertools;
use std::rc::Rc;

/// Calcuate the reads-from and writes-to set for a given set of assignments.
pub struct ReadWriteSet;

impl ReadWriteSet {
    /// Returns [ir::Port] which are read from in the assignments.
    pub fn port_read_set(assigns: &[ir::Assignment]) -> ir::PortIterator<'_> {
        let guard_ports = assigns
            .iter()
            .flat_map(|assign| assign.guard.all_ports().into_iter());
        let iter = assigns
            .iter()
            .map(|assign| Rc::clone(&assign.src))
            .chain(guard_ports)
            .filter(|port| {
                matches!(port.borrow().parent, ir::PortParent::Cell(_))
            });
        ir::PortIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Returns [ir::Port] which are written to in the assignments.
    pub fn port_write_set(assigns: &[ir::Assignment]) -> ir::PortIterator<'_> {
        let iter = assigns.iter().map(|assign| Rc::clone(&assign.dst)).filter(
            |port| matches!(port.borrow().parent, ir::PortParent::Cell(_)),
        );
        ir::PortIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Returns [ir::Cell] which are read from in the assignments.
    /// **Ignores** reads from group holes.
    pub fn read_set(assigns: &[ir::Assignment]) -> ir::CellIterator<'_> {
        let iter = Self::port_read_set(assigns)
            .map(|port_ref| {
                let port = port_ref.borrow();
                if let ir::PortParent::Cell(cell_wref) = &port.parent {
                    Rc::clone(&cell_wref.upgrade())
                } else {
                    unreachable!()
                }
            })
            .unique_by(|cell| cell.clone_name());

        ir::CellIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Returns [ir::Cell] which are written to by the assignments.
    /// **Ignores** reads from group holes.
    pub fn write_set(assigns: &[ir::Assignment]) -> ir::CellIterator<'_> {
        let iter = Self::port_write_set(assigns)
            .map(|port_ref| {
                let port = port_ref.borrow();
                if let ir::PortParent::Cell(cell_wref) = &port.parent {
                    Rc::clone(&cell_wref.upgrade())
                } else {
                    unreachable!()
                }
            })
            .unique_by(|cell| cell.clone_name());

        ir::CellIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Returns the register cells whose out port is read anywhere in the given
    /// assignments
    pub fn register_reads(assigns: &[ir::Assignment]) -> ir::CellIterator<'_> {
        let guard_ports = assigns.iter().flat_map(|assign| {
            assign.guard.all_ports().into_iter().filter_map(|port_ref| {
                let port = port_ref.borrow();
                if let ir::PortParent::Cell(cell_wref) = &port.parent {
                    if &port.name == "out" {
                        return Some(Rc::clone(&cell_wref.upgrade()));
                    }
                }
                None
            })
        });
        let iter = assigns
            .iter()
            .filter_map(|assign| {
                let src_ref = assign.src.borrow();
                if let ir::PortParent::Cell(cell_wref) = &src_ref.parent {
                    if src_ref.name == "out" {
                        return Some(Rc::clone(&cell_wref.upgrade()));
                    }
                }
                None
            })
            .chain(guard_ports)
            .filter(|x| {
                if let Some(name) = x.borrow().type_name() {
                    name == "std_reg"
                } else {
                    false
                }
            })
            .unique_by(|cell| cell.clone_name());

        ir::CellIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Return the name of the cells that these assignments write to for writes
    /// that are guarded by true.
    /// **Ignores** writes to group holes.
    pub fn must_write_set(assigns: &[ir::Assignment]) -> ir::CellIterator<'_> {
        let iter = assigns
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
            .unique_by(|cell| cell.clone_name());

        ir::CellIterator {
            port_iter: Box::new(iter),
        }
    }

    /// Returns all uses of cells in this group. Uses constitute both reads and
    /// writes to cells.
    pub fn uses(assigns: &[ir::Assignment]) -> ir::CellIterator<'_> {
        let reads = Self::read_set(assigns);
        let iter = reads
            .chain(Self::write_set(assigns))
            .unique_by(|cell| cell.clone_name());

        ir::CellIterator {
            port_iter: Box::new(iter),
        }
    }
}
