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
    pub fn port_read_set<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + 'a,
    ) -> impl Iterator<Item = RRC<ir::Port>> + 'a {
        assigns.flat_map(Self::port_reads)
    }

    /// Returns [ir::Port] which are written to in the assignments.
    pub fn port_write_set<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + 'a,
    ) -> impl Iterator<Item = RRC<ir::Port>> + 'a {
        assigns
            .map(|assign| Rc::clone(&assign.dst))
            .filter(|port| !port.borrow().is_hole())
    }

    /// Returns [ir::Cell] which are read from in the assignments.
    /// **Ignores** reads from group holes.
    pub fn read_set<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + 'a,
    ) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        Self::port_read_set(assigns)
            .map(|port| Rc::clone(&port.borrow().cell_parent()))
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns [ir::Cell] which are written to by the assignments.
    /// **Ignores** reads from group holes.
    pub fn write_set<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + 'a,
    ) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        Self::port_write_set(assigns)
            .map(|port| Rc::clone(&port.borrow().cell_parent()))
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns the register cells whose out port is read anywhere in the given
    /// assignments
    pub fn register_reads<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + Clone + 'a,
    ) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        fn is_register_out(port_ref: RRC<ir::Port>) -> Option<RRC<ir::Cell>> {
            let port = port_ref.borrow();
            if let ir::PortParent::Cell(cell_wref) = &port.parent {
                if &port.name == "out" {
                    return Some(Rc::clone(&cell_wref.upgrade()));
                }
            }
            None
        }
        let guard_ports = assigns.clone().flat_map(|assign| {
            assign
                .guard
                .all_ports()
                .into_iter()
                .filter_map(is_register_out)
        });
        assigns
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
    pub fn must_write_set<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + 'a,
    ) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        assigns
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
    pub fn uses<'a>(
        assigns: impl Iterator<Item = &'a ir::Assignment> + Clone + 'a,
    ) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        let reads = Self::read_set(assigns.clone());
        reads
            .chain(Self::write_set(assigns))
            .unique_by(|cell| cell.clone_name())
    }

    /// Returns the ports that are read by the given control program.
    pub fn control_port_read_set(con: &ir::Control) -> Vec<RRC<ir::Port>> {
        match con {
            ir::Control::Empty(_) => vec![],
            ir::Control::Enable(ir::Enable { group, .. }) => {
                Self::port_read_set(group.borrow().assignments.iter()).collect()
            }
            ir::Control::Invoke(ir::Invoke {
                inputs, comb_group, ..
            }) => {
                let inps = inputs.iter().map(|(_, p)| p).cloned();
                match comb_group {
                    Some(cg) => {
                        Self::port_read_set(cg.borrow().assignments.iter())
                            .chain(inps)
                            .collect()
                    }
                    None => inps.collect(),
                }
            }

            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                stmts.iter().flat_map(Self::control_port_read_set).collect()
            }
            ir::Control::If(ir::If {
                port,
                cond,
                tbranch,
                fbranch,
                ..
            }) => {
                let common = Self::control_port_read_set(tbranch)
                    .into_iter()
                    .chain(Self::control_port_read_set(fbranch).into_iter())
                    .chain(std::iter::once(Rc::clone(port)));
                match cond {
                    Some(cg) => {
                        Self::port_read_set(cg.borrow().assignments.iter())
                            .chain(common)
                            .collect()
                    }
                    None => common.collect(),
                }
            }
            ir::Control::While(ir::While {
                port, cond, body, ..
            }) => {
                let common = Self::control_port_read_set(body)
                    .into_iter()
                    .chain(std::iter::once(Rc::clone(port)));
                match cond {
                    Some(cg) => {
                        Self::port_read_set(cg.borrow().assignments.iter())
                            .chain(common)
                            .collect()
                    }
                    None => common.collect(),
                }
            }
        }
    }
}
