use crate::ir::{self, RRC};
use std::rc::Rc;

/// Calcuate the reads-from and writes-to set for a given set of assignments.
pub struct ReadWriteSet {}

impl ReadWriteSet {
    /// Returns the name of the cells these assignments read from.
    /// **Ignores** reads from group holes.
    pub fn read_set(assigns: &[ir::Assignment]) -> Vec<RRC<ir::Cell>> {
        assigns
            .into_iter()
            .filter_map(|assign| {
                let src_ref = assign.src.borrow();
                if let ir::PortParent::Cell(cell_wref) = &src_ref.parent {
                    Some(Rc::clone(&cell_wref.upgrade().unwrap()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the name of the cells these assignments write to.
    /// **Ignores** reads from group holes.
    pub fn write_set(assigns: &[ir::Assignment]) -> Vec<RRC<ir::Cell>> {
        assigns
            .into_iter()
            .filter_map(|assign| {
                let dst_ref = assign.dst.borrow();
                if let ir::PortParent::Cell(cell_wref) = &dst_ref.parent {
                    Some(Rc::clone(&cell_wref.upgrade().unwrap()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns all uses of cells in this group. Uses constitute both reads and
    /// writes to cells.
    pub fn uses(assigns: &[ir::Assignment]) -> Vec<RRC<ir::Cell>> {
        let mut reads = Self::read_set(assigns);
        reads.append(&mut Self::write_set(assigns));
        reads
    }
}
