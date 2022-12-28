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
}

impl ReadWriteSet {
    /// Returns the ports that are read by the given control program.
    pub fn control_port_read_write_set(
        con: &ir::Control,
    ) -> (Vec<RRC<ir::Port>>, Vec<RRC<ir::Port>>) {
        match con {
            ir::Control::Empty(_) => (vec![], vec![]),
            ir::Control::Enable(ir::Enable { group, .. }) => (
                Self::port_read_set(group.borrow().assignments.iter())
                    .collect(),
                Self::port_write_set(group.borrow().assignments.iter())
                    .collect(),
            ),
            ir::Control::Invoke(ir::Invoke {
                inputs, comb_group, ..
            }) => {
                let inps = inputs.iter().map(|(_, p)| p).cloned();
                let outs = inputs.iter().map(|(_, p)| p).cloned();
                match comb_group {
                    Some(cgr) => {
                        let cg = cgr.borrow();
                        let assigns = cg.assignments.iter();
                        let reads = Self::port_read_set(assigns.clone());
                        let writes = Self::port_write_set(assigns);
                        (
                            reads.chain(inps).collect(),
                            writes.chain(outs).collect(),
                        )
                    }
                    None => (inps.collect(), outs.collect()),
                }
            }

            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                for stmt in stmts {
                    let (mut read, mut write) =
                        Self::control_port_read_write_set(stmt);
                    reads.append(&mut read);
                    writes.append(&mut write);
                }
                (reads, writes)
            }
            ir::Control::If(ir::If {
                port,
                cond,
                tbranch,
                fbranch,
                ..
            }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                let (mut treads, mut twrites) =
                    Self::control_port_read_write_set(tbranch);
                let (mut freads, mut fwrites) =
                    Self::control_port_read_write_set(fbranch);
                reads.append(&mut treads);
                reads.append(&mut freads);
                reads.push(Rc::clone(port));
                writes.append(&mut twrites);
                writes.append(&mut fwrites);

                if let Some(cg) = cond {
                    reads.extend(Self::port_read_set(
                        cg.borrow().assignments.iter(),
                    ));
                    writes.extend(Self::port_write_set(
                        cg.borrow().assignments.iter(),
                    ));
                }
                (reads, writes)
            }
            ir::Control::While(ir::While {
                port, cond, body, ..
            }) => {
                let (mut reads, mut writes) =
                    Self::control_port_read_write_set(body);
                reads.push(Rc::clone(port));

                if let Some(cg) = cond {
                    reads.extend(Self::port_read_set(
                        cg.borrow().assignments.iter(),
                    ));
                    writes.extend(Self::port_write_set(
                        cg.borrow().assignments.iter(),
                    ));
                }
                (reads, writes)
            }
        }
    }
}

trait Unique {
    /// The unique index type for this type.
    type T: Eq + Clone + core::hash::Hash;

    /// The canonical name for this type
    fn unique(&self) -> Self::T;
}

impl Unique for ir::Cell {
    type T = ir::Id;

    fn unique(&self) -> Self::T {
        self.clone_name()
    }
}

impl Unique for ir::Port {
    type T = ir::Canonical;

    fn unique(&self) -> Self::T {
        self.canonical()
    }
}

/// A trait for types that make use of a type `T`.
trait Uses<T> {
    /// Reads of type T from this type. Not guaranteed to be unique.
    fn reads(&self) -> Vec<RRC<T>>;
    /// Writes to type T in this type. Not guaranteed to be unique.
    fn writes(&self) -> Vec<RRC<T>>;

    /// Return the read and the write set. This method is exposed because it is
    /// cheaper to compute both sets  at the same time on some types.
    fn read_write_sets(&self) -> (Vec<RRC<T>>, Vec<RRC<T>>) {
        (self.reads(), self.writes())
    }

    /// Returns all uses of ports in this type constituting both reads and writes
    fn uses(&self) -> Vec<RRC<T>> {
        let (mut reads, writes) = self.read_write_sets();
        reads.extend(writes);
        reads
    }
}

impl Uses<ir::Port> for ir::Assignment {
    fn reads(&self) -> Vec<RRC<ir::Port>> {
        self.guard
            .all_ports()
            .into_iter()
            .chain(iter::once(Rc::clone(&self.src)))
            .filter(|port| !port.borrow().is_hole())
            .collect_vec()
    }

    fn writes(&self) -> Vec<RRC<ir::Port>> {
        vec![Rc::clone(&self.dst)]
    }
}

impl Uses<ir::Port> for ir::Group {
    fn reads(&self) -> Vec<RRC<ir::Port>> {
        self.assignments
            .iter()
            .flat_map(|assign| {
                <ir::Assignment as Uses<ir::Port>>::reads(assign)
            })
            .chain(iter::once(Rc::clone(&self.done_cond)))
            .collect_vec()
    }

    fn writes(&self) -> Vec<RRC<ir::Port>> {
        self.assignments
            .iter()
            .flat_map(|assign| {
                <ir::Assignment as Uses<ir::Port>>::writes(assign)
            })
            .collect_vec()
    }
}

impl Uses<ir::Port> for ir::CombGroup {
    fn reads(&self) -> Vec<RRC<ir::Port>> {
        self.assignments
            .iter()
            .flat_map(|assign| {
                <ir::Assignment as Uses<ir::Port>>::reads(assign)
            })
            .collect_vec()
    }

    fn writes(&self) -> Vec<RRC<ir::Port>> {
        self.assignments
            .iter()
            .flat_map(|assign| {
                <ir::Assignment as Uses<ir::Port>>::writes(assign)
            })
            .collect_vec()
    }
}

impl Uses<ir::Port> for ir::Control {
    fn read_write_sets(&self) -> (Vec<RRC<ir::Port>>, Vec<RRC<ir::Port>>) {
        match self {
            ir::Control::Empty(_) => (vec![], vec![]),
            ir::Control::Enable(ir::Enable { group, .. }) => {
                group.borrow().read_write_sets()
            }
            ir::Control::Invoke(ir::Invoke {
                inputs,
                outputs,
                ref_cells,
                comb_group,
                ..
            }) => {
                let mut inps =
                    inputs.iter().map(|(_, p)| p).cloned().collect_vec();
                let mut outs =
                    outputs.iter().map(|(_, p)| p).cloned().collect_vec();
                match comb_group {
                    Some(cgr) => {
                        let (mut reads, mut writes) =
                            cgr.borrow().read_write_sets();
                        inps.append(&mut reads);
                        outs.append(&mut writes);
                    }
                    None => (),
                }
                // All ports defined on the ref cells are uses
                for (_, cell) in ref_cells {
                    for port in &cell.borrow().ports {
                        match &port.borrow().direction {
                            ir::Direction::Input => inps.push(Rc::clone(port)),
                            ir::Direction::Output => outs.push(Rc::clone(port)),
                            ir::Direction::Inout => unreachable!(),
                        }
                    }
                }
                (inps, outs)
            }

            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                for stmt in stmts {
                    let (mut read, mut write) = stmt.read_write_sets();
                    reads.append(&mut read);
                    writes.append(&mut write);
                }
                (reads, writes)
            }
            ir::Control::If(ir::If {
                port,
                cond,
                tbranch,
                fbranch,
                ..
            }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                let (mut treads, mut twrites) = tbranch.read_write_sets();
                let (mut freads, mut fwrites) = fbranch.read_write_sets();
                reads.append(&mut treads);
                reads.append(&mut freads);
                reads.push(Rc::clone(port));
                writes.append(&mut twrites);
                writes.append(&mut fwrites);

                if let Some(cg) = cond {
                    let (cg_reads, cg_writes) = cg.borrow().read_write_sets();
                    reads.extend(cg_reads);
                    writes.extend(cg_writes);
                }
                (reads, writes)
            }
            ir::Control::While(ir::While {
                port, cond, body, ..
            }) => {
                let (mut reads, mut writes) = body.read_write_sets();
                reads.push(Rc::clone(port));

                if let Some(cg) = cond {
                    let (cg_reads, cg_writes) = cg.borrow().read_write_sets();
                    reads.extend(cg_reads);
                    writes.extend(cg_writes);
                }
                (reads, writes)
            }
        }
    }

    fn reads(&self) -> Vec<RRC<ir::Port>> {
        self.read_write_sets().0
    }

    fn writes(&self) -> Vec<RRC<ir::Port>> {
        self.read_write_sets().1
    }
}

impl Uses<ir::Port> for ir::Component {
    fn read_write_sets(&self) -> (Vec<RRC<ir::Port>>, Vec<RRC<ir::Port>>) {
        let (mut reads, mut writes) = (vec![], vec![]);
        // The read and writes from all the groups
        for gr in self.groups.iter() {
            let (mut gr_reads, mut gr_writes) = gr.borrow().read_write_sets();
            reads.append(&mut gr_reads);
            writes.append(&mut gr_writes);
        }

        for cg in self.comb_groups.iter() {
            let (mut cg_reads, mut cg_writes) = cg.borrow().read_write_sets();
            reads.append(&mut cg_reads);
            writes.append(&mut cg_writes);
        }

        // The read and writes from the control
        let (mut ctrl_reads, mut ctrl_writes) =
            self.control.borrow().read_write_sets();
        reads.append(&mut ctrl_reads);
        writes.append(&mut ctrl_writes);

        (reads, writes)
    }

    fn reads(&self) -> Vec<RRC<ir::Port>> {
        self.read_write_sets().0
    }

    fn writes(&self) -> Vec<RRC<ir::Port>> {
        self.read_write_sets().1
    }
}

impl<T> Uses<ir::Cell> for T
where
    T: Uses<ir::Port>,
{
    fn reads(&self) -> Vec<RRC<ir::Cell>> {
        self.reads()
            .iter()
            .map(|port| port.borrow().cell_parent())
            .unique_by(|cell| cell.borrow().name())
            .collect_vec()
    }

    fn writes(&self) -> Vec<RRC<ir::Cell>> {
        self.writes()
            .iter()
            .map(|port| port.borrow().cell_parent())
            .unique_by(|cell| cell.borrow().name())
            .collect_vec()
    }
}
