use std::{cmp::Ordering, collections::HashSet};

use itertools::Itertools;

use super::read_write_set::ReadWriteSet;
use crate::ir::{self, CloneName};

type ReadWriteMap = Vec<(HashSet<ir::Id>, HashSet<ir::Id>)>;

/// Given a set of assignment, generates an ordering that respects combinatinal
/// dataflow.
pub struct DataflowOrder {
    /// Mapping from the index of the assignment to its read and write set.
    read_write_map: ReadWriteMap,
}

/// Returns true iff the given port is a sequential output and therefore should
/// not be used in the ordering decision.
fn is_seq_port(port: &ir::Port) -> bool {
    port.direction == ir::Direction::Output
        && port.attributes.get("stable").is_some()
}

impl DataflowOrder {
    /// Precompute the read and write sets for each assignment in a slice.
    fn compute_read_write_map(assigns: &[ir::Assignment]) -> ReadWriteMap {
        let mut map = Vec::with_capacity(assigns.len());
        for assign in assigns {
            // Add dst to write port only if the parent is a cell.
            let dst = assign.dst.borrow();
            let write_port = if let ir::PortParent::Cell(_) = &dst.parent {
                Some(&*dst)
            } else {
                None
            };
            let write: HashSet<ir::Id> = write_port
                .into_iter()
                .filter(|port| is_seq_port(port))
                .map(|pr| pr.cell_parent().borrow().clone_name())
                .collect();

            let reads = ReadWriteSet::port_reads(assign)
                .filter_map(|port_ref| {
                    let port = port_ref.borrow();
                    if is_seq_port(&port) {
                        None
                    } else {
                        Some(port.cell_parent().borrow().clone_name())
                    }
                })
                .unique()
                .collect::<HashSet<_>>();

            map.push((reads, write))
        }
        map
    }

    fn dataflow_sort(assigns: Vec<ir::Assignment>) -> Vec<ir::Assignment> {
        let rw_map = Self::compute_read_write_map(&assigns);
        assigns.into_iter().enumerate().sorted_by(
            |(a1_idx, a1), (a2_idx, a2)| {
                let (a1r, a1w) = &rw_map[*a1_idx];
                let (a2r, a2w) = &rw_map[*a2_idx];
                let overlap_a1 = a1w.intersection(a2r).count();
                let overlap_a2 = a2w.intersection(a1r).count();
                if overlap_a1 != 0 && overlap_a2 == 0 {
                    Ordering::Less
                } else if overlap_a2 != 0 && overlap_a1 == 0 {
                    Ordering::Greater
                } else if overlap_a1 == 0 && overlap_a2 == 0 {
                    Ordering::Equal
                } else {
                    panic!(
                        "No dataflow ordering. Found combinational loop between assignments.\n{}\t{} (reads), {} (writes)\n{}\t{} (reads), {} (writes)",
                        ir::Printer::assignment_to_str(a1),
                        a1r.iter().cloned().map(|id| id.id).join(", "),
                        a1w.iter().cloned().map(|id| id.id).join(", "),
                        ir::Printer::assignment_to_str(a2),
                        a2r.iter().cloned().map(|id| id.id).join(", "),
                        a2w.iter().cloned().map(|id| id.id).join(", "),
                    )
                }
            },
        ).map(|(_, a)| a).collect()
    }
}
