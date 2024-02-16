use calyx_ir::{self as ir, RRC};
use itertools::Itertools;
use std::{collections::HashMap, iter, rc::Rc};

#[derive(Clone)]
pub struct AssignmentIterator<'a, T: 'a, I>
where
    I: Iterator<Item = &'a ir::Assignment<T>>,
{
    iter: I,
}

impl<'a, T: 'a, I> Iterator for AssignmentIterator<'a, T, I>
where
    I: Iterator<Item = &'a ir::Assignment<T>>,
{
    type Item = &'a ir::Assignment<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a, T: 'a, I: 'a> AssignmentIterator<'a, T, I>
where
    I: Iterator<Item = &'a ir::Assignment<T>>,
{
    /// Returns [ir::Port] which are read from in the assignments.
    pub fn reads(
        self,
    ) -> PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a> {
        PortIterator::new(self.flat_map(ReadWriteSet::port_reads))
    }

    /// Returns [ir::Port] which are written to in the assignments.
    pub fn writes(
        self,
    ) -> PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a> {
        PortIterator::new(
            self.map(|assign| Rc::clone(&assign.dst))
                .filter(|port| !port.borrow().is_hole()),
        )
    }

    /// Return the name of the cells that these assignments write to for writes
    /// that are guarded by true.
    /// **Ignores** writes to group holes.
    pub fn must_writes(
        self,
    ) -> PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a> {
        PortIterator::new(self.filter_map(|assignment| {
            if assignment.guard.is_true() && !assignment.dst.borrow().is_hole()
            {
                Some(Rc::clone(&assignment.dst))
            } else {
                None
            }
        }))
    }

    /// Returns the ports mentioned in this set of assignments.
    pub fn uses(
        self,
    ) -> PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a> {
        PortIterator::new(self.flat_map(|assign| {
            assign
                .guard
                .all_ports()
                .into_iter()
                .chain(iter::once(Rc::clone(&assign.dst)))
                .chain(iter::once(Rc::clone(&assign.src)))
                .filter(|port| !port.borrow().is_hole())
        }))
    }

    // Convinience Methods

    /// Returns the cells read from in this set of assignments
    pub fn cell_reads(self) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        self.reads().cells()
    }

    /// Returns the cells written to in this set of assignments
    pub fn cell_writes(self) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        self.writes().cells()
    }

    /// Returns the cells used in this set of assignments
    pub fn cell_uses(self) -> impl Iterator<Item = RRC<ir::Cell>> + 'a {
        self.uses().cells()
    }
}

impl<'a, T: 'a, I: 'a> AssignmentIterator<'a, T, I>
where
    I: Iterator<Item = &'a ir::Assignment<T>>,
    I: Clone,
    T: Clone,
{
    /// Separately returns the read and write sets for the given assignments.
    pub fn reads_and_writes(
        self,
    ) -> (
        PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a>,
        PortIterator<impl Iterator<Item = RRC<ir::Port>> + 'a>,
    ) {
        (self.clone().reads(), self.writes())
    }
}

/// Analyzes that can be performed on a set of assignments.
pub trait AssignmentAnalysis<'a, T: 'a>:
    Iterator<Item = &'a ir::Assignment<T>>
where
    Self: Sized,
{
    fn analysis(self) -> AssignmentIterator<'a, T, Self> {
        AssignmentIterator { iter: self }
    }
}

impl<'a, T: 'a, I: 'a> AssignmentAnalysis<'a, T> for I where
    I: Iterator<Item = &'a ir::Assignment<T>>
{
}

/// An iterator over ports
pub struct PortIterator<I>
where
    I: Iterator<Item = RRC<ir::Port>>,
{
    iter: I,
}

impl<I> Iterator for PortIterator<I>
where
    I: Iterator<Item = RRC<ir::Port>>,
{
    type Item = RRC<ir::Port>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<I> PortIterator<I>
where
    I: Iterator<Item = RRC<ir::Port>>,
{
    pub const fn new(iter: I) -> Self {
        Self { iter }
    }

    /// Return the unique cells that the ports are a part of
    pub fn cells(self) -> impl Iterator<Item = RRC<ir::Cell>> {
        self.iter
            .map(|port| Rc::clone(&port.borrow().cell_parent()))
            .unique_by(|cell| cell.borrow().name())
    }

    /// Group the ports by cells that they are a part of
    pub fn group_by_cell(self) -> HashMap<ir::Id, Vec<RRC<ir::Port>>> {
        self.iter.into_group_map_by(|port| {
            port.borrow().cell_parent().borrow().name()
        })
    }
}

/// Calcuate the reads-from and writes-to set for a given set of assignments.
pub struct ReadWriteSet;

impl ReadWriteSet {
    /// Returns [ir::Port] that are read from in the given Assignment.
    pub fn port_reads<T>(
        assign: &ir::Assignment<T>,
    ) -> PortIterator<impl Iterator<Item = RRC<ir::Port>>> {
        PortIterator::new(
            assign
                .guard
                .all_ports()
                .into_iter()
                .chain(iter::once(Rc::clone(&assign.src)))
                .filter(|port| !port.borrow().is_hole()),
        )
    }
}

impl ReadWriteSet {
    /// Returns the ports that are read and written, respectively,
    /// by the given static control program.
    pub fn control_port_read_write_set_static(
        scon: &ir::StaticControl,
    ) -> (Vec<RRC<ir::Port>>, Vec<RRC<ir::Port>>) {
        match scon {
            ir::StaticControl::Empty(_) => (vec![], vec![]),
            ir::StaticControl::Enable(ir::StaticEnable { group, .. }) => {
                let g = group.borrow();
                let (r, w) = g.assignments.iter().analysis().reads_and_writes();
                (r.collect(), w.collect())
            }
            ir::StaticControl::Repeat(ir::StaticRepeat { body, .. }) => {
                Self::control_port_read_write_set_static(body)
            }
            ir::StaticControl::Seq(ir::StaticSeq { stmts, .. })
            | ir::StaticControl::Par(ir::StaticPar { stmts, .. }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                for stmt in stmts {
                    let (mut read, mut write) =
                        Self::control_port_read_write_set_static(stmt);
                    reads.append(&mut read);
                    writes.append(&mut write);
                }
                (reads, writes)
            }
            ir::StaticControl::If(ir::StaticIf {
                port,
                tbranch,
                fbranch,
                ..
            }) => {
                let (mut treads, mut twrites) =
                    Self::control_port_read_write_set_static(tbranch);
                let (mut freads, mut fwrites) =
                    Self::control_port_read_write_set_static(fbranch);
                treads.append(&mut freads);
                treads.push(Rc::clone(port));
                twrites.append(&mut fwrites);

                (treads, twrites)
            }
            ir::StaticControl::Invoke(ir::StaticInvoke {
                inputs,
                outputs,
                ref_cells,
                comp,
                ..
            }) => {
                let mut inps: Vec<RRC<ir::Port>> =
                    inputs.iter().map(|(_, p)| p).cloned().collect();
                let mut outs: Vec<RRC<ir::Port>> =
                    outputs.iter().map(|(_, p)| p).cloned().collect();
                // Adding comp.go to input ports
                inps.push(
                    comp.borrow()
                        .find_all_with_attr(ir::NumAttr::Go)
                        .next()
                        .unwrap_or_else(|| {
                            unreachable!(
                                "No @go port for component {}",
                                comp.borrow().name()
                            )
                        }),
                );
                for (_, cell) in ref_cells.iter() {
                    for port in cell.borrow().ports.iter() {
                        match port.borrow().direction {
                            ir::Direction::Input => {
                                outs.push(Rc::clone(port));
                            }
                            ir::Direction::Output => {
                                inps.push(Rc::clone(port));
                            }
                            _ => {
                                outs.push(Rc::clone(port));
                                inps.push(Rc::clone(port));
                            }
                        }
                    }
                }
                (inps, outs)
            }
        }
    }

    /// Returns the cells that are read and written, respectively,
    /// by the given static control program.
    pub fn control_read_write_set_static(
        scon: &ir::StaticControl,
    ) -> (Vec<RRC<ir::Cell>>, Vec<RRC<ir::Cell>>) {
        let (port_reads, port_writes) =
            Self::control_port_read_write_set_static(scon);
        (
            port_reads
                .into_iter()
                .map(|p| p.borrow().cell_parent())
                .collect(),
            port_writes
                .into_iter()
                .map(|p| p.borrow().cell_parent())
                .collect(),
        )
    }

    /// Returns the ports that are read and written, respectively,
    /// by the given control program.
    /// INCLUDE_HOLE_ASSIGNS: in either case, we will ignore done holes.
    /// However, if INCLUDE_HOLE_ASSIGNS is false, we ignore all *assignments*
    /// that write to holes, even if the src of that assignment is a cell's port.
    pub fn control_port_read_write_set<const INCLUDE_HOLE_ASSIGNS: bool>(
        con: &ir::Control,
    ) -> (Vec<RRC<ir::Port>>, Vec<RRC<ir::Port>>) {
        match con {
            ir::Control::Empty(_) => (vec![], vec![]),
            ir::Control::Enable(ir::Enable { group, .. }) => {
                let group = group.borrow();
                let (reads, writes) =
                    group.assignments.iter().analysis().reads_and_writes();
                (
                    reads
                        .filter(|p| {
                            INCLUDE_HOLE_ASSIGNS || !p.borrow().is_hole()
                        })
                        .collect(),
                    writes
                        .filter(|p| {
                            INCLUDE_HOLE_ASSIGNS || !p.borrow().is_hole()
                        })
                        .collect(),
                )
            }
            ir::Control::Invoke(ir::Invoke {
                inputs,
                outputs,
                comb_group,
                ref_cells,
                comp,
                ..
            }) => {
                // XXX(Caleb): Maybe check that there is one @go port.
                let inps = inputs.iter().map(|(_, p)| p).cloned();
                let outs = outputs.iter().map(|(_, p)| p).cloned();
                let mut r: Vec<RRC<ir::Port>> = inps.collect();
                let mut w: Vec<RRC<ir::Port>> = outs.collect();
                // Adding comp.go to the write set
                w.push(
                    comp.borrow()
                        .find_all_with_attr(ir::NumAttr::Go)
                        .next()
                        .unwrap_or_else(|| {
                            unreachable!(
                                "No @go port for component {}",
                                comp.borrow().name()
                            )
                        }),
                );

                for (_, cell) in ref_cells {
                    for port in cell.borrow().ports.iter() {
                        match port.borrow().direction {
                            ir::Direction::Input => {
                                w.push(Rc::clone(port));
                            }
                            ir::Direction::Output => {
                                r.push(Rc::clone(port));
                            }
                            _ => {
                                w.push(Rc::clone(port));
                                r.push(Rc::clone(port));
                            }
                        }
                    }
                }
                match comb_group {
                    Some(cgr) => {
                        let cg = cgr.borrow();
                        let (reads, writes) =
                            cg.assignments.iter().analysis().reads_and_writes();
                        (
                            reads.into_iter().chain(r).collect(),
                            writes.into_iter().chain(w).collect(),
                        )
                    }
                    None => (r, w),
                }
            }
            ir::Control::Seq(ir::Seq { stmts, .. })
            | ir::Control::Par(ir::Par { stmts, .. }) => {
                let (mut reads, mut writes) = (vec![], vec![]);
                for stmt in stmts {
                    let (mut read, mut write) =
                        Self::control_port_read_write_set::<true>(stmt);
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
                let (mut treads, mut twrites) =
                    Self::control_port_read_write_set::<true>(tbranch);
                let (mut freads, mut fwrites) =
                    Self::control_port_read_write_set::<true>(fbranch);
                treads.append(&mut freads);
                treads.push(Rc::clone(port));
                twrites.append(&mut fwrites);

                if let Some(cg) = cond {
                    let cg = cg.borrow();
                    let (reads, writes) =
                        cg.assignments.iter().analysis().reads_and_writes();
                    treads.extend(reads);
                    twrites.extend(writes);
                }
                (treads, twrites)
            }
            ir::Control::While(ir::While {
                port, cond, body, ..
            }) => {
                let (mut reads, mut writes) =
                    Self::control_port_read_write_set::<true>(body);
                reads.push(Rc::clone(port));

                if let Some(cg) = cond {
                    let cg = cg.borrow();
                    let (r, w) =
                        cg.assignments.iter().analysis().reads_and_writes();
                    reads.extend(r);
                    writes.extend(w);
                }
                (reads, writes)
            }
            ir::Control::Repeat(ir::Repeat { body, .. }) => {
                Self::control_port_read_write_set::<true>(body)
            }
            ir::Control::Static(sc) => {
                Self::control_port_read_write_set_static(sc)
            }
        }
    }

    /// Returns the cells that are read and written, respectively,
    /// by the given control program.
    /// INCLUDE_HOLE_ASSIGNS: in either case, we will ignore done holes themselves.
    /// However, if INCLUDE_HOLE_ASSIGNS is true, we ignore all assignments
    /// that write to holes, even if the src of that assignment is a cell's port.
    pub fn control_read_write_set<const INCLUDE_HOLE_ASSIGNS: bool>(
        con: &ir::Control,
    ) -> (Vec<RRC<ir::Cell>>, Vec<RRC<ir::Cell>>) {
        let (port_reads, port_writes) =
            Self::control_port_read_write_set::<INCLUDE_HOLE_ASSIGNS>(con);
        (
            port_reads
                .into_iter()
                .map(|p| p.borrow().cell_parent())
                .collect(),
            port_writes
                .into_iter()
                .map(|p| p.borrow().cell_parent())
                .collect(),
        )
    }
}
