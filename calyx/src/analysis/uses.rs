use std::{iter, rc::Rc};

use itertools::Itertools;

use crate::ir::{self, CloneName, RRC};

/// A trait for types that make use of a type `T`.
pub trait Uses<T> {
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
                // All ports defined on the ref cells are assumed to be used.
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
                let (mut reads, mut writes) = tbranch.read_write_sets();
                let (mut freads, mut fwrites) = fbranch.read_write_sets();
                reads.append(&mut freads);
                writes.append(&mut fwrites);
                reads.push(Rc::clone(port));

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

/// A type that has a unique name per instance.
pub trait Unique {
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

/// A type that uses type `T` and can be used to compute the unique uses of `T`.
pub trait UniqueUses<T> {
    fn unique_reads(&self) -> Vec<RRC<T>>;
    fn unique_writes(&self) -> Vec<RRC<T>>;
}

impl<T, I> UniqueUses<I> for T
where
    T: Uses<I>,
    I: Unique,
{
    fn unique_reads(&self) -> Vec<RRC<I>> {
        self.reads()
            .into_iter()
            .unique_by(|i| i.borrow().unique())
            .collect()
    }

    fn unique_writes(&self) -> Vec<RRC<I>> {
        self.writes()
            .into_iter()
            .unique_by(|i| i.borrow().unique())
            .collect()
    }
}
