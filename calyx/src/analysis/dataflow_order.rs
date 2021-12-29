use std::collections::HashSet;

use itertools::Itertools;

use super::read_write_set::ReadWriteSet;
use crate::ir::{self, CloneName};

/// Given a set of assignment, generates an ordering that respects combinatinal
/// dataflow.
struct DataflowOrder {
    /// Mapping from the index of the assignment to its read and write set.
    read_write_map: Vec<(HashSet<ir::Id>, Option<ir::Id>)>,
}

impl From<&[ir::Assignment]> for DataflowOrder {
    fn from(assigns: &[ir::Assignment]) -> Self {
        let mut map = Vec::with_capacity(assigns.len());
        for assign in assigns {
            let write = if let ir::PortParent::Cell(cell_wref) =
                &assign.dst.borrow().parent
            {
                Some(cell_wref.upgrade().clone_name())
            } else {
                None
            };

            let reads = ReadWriteSet::port_reads(assign)
                .map(|port_ref| {
                    port_ref.borrow().cell_parent().borrow().clone_name()
                })
                .unique()
                .collect::<HashSet<_>>();

            map.push((reads, write))
        }

        DataflowOrder {
            read_write_map: map,
        }
    }
}
