use std::rc::Rc;

use itertools::Itertools;

use crate::ir::{self, CloneName, RRC};

/// A trait for types that make use (read from or write to) of a type `T`. Implemented for [ir::Cell] and [ir::Port].
/// Example:
/// ```
/// fn cell_and_ports(assigns: &[ir::Assignment]) -> (Vec<RRC<ir::Cell>>, Vec<RRC<ir::Port>>) {
///   let cell_reads = Uses::<ir::Cell>::reads(assigns.iter());
///   let port_writes = Uses::<ir::Port>::writes(assigns.iter());
///   (cell_reads, port_writes)
/// }
/// ```
///
/// Provides convience methods to optimize the number of iterations over the
/// type when computing the read and write sets.
/// The in-place methods are useful to avoid reallocation of the underlying
/// vectors when possible.
pub trait Uses<T> {
    /// Compute the set of reads of type `T` in this type and add them to the given vector.
    /// Not guaranteed to be unique.
    fn reads_in_place(&self, reads: &mut Vec<RRC<T>>);

    /// Compute the set of writes of type `T` in this type and add them to the given vector.
    /// Not guaranteed to be unique.
    fn writes_in_place(&self, writes: &mut Vec<RRC<T>>);

    /// Compute both the reads and writes. Implementing types should override this method
    /// if it is cheaper to compute both sets at the same time.
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<T>>,
        writes: &mut Vec<RRC<T>>,
    ) {
        self.reads_in_place(reads);
        self.writes_in_place(writes);
    }

    /// Reads of type T from this type. Not guaranteed to be unique.
    fn reads(&self) -> Vec<RRC<T>> {
        let mut reads = Vec::new();
        self.reads_in_place(&mut reads);
        reads
    }
    /// Writes to type T in this type. Not guaranteed to be unique.
    fn writes(&self) -> Vec<RRC<T>> {
        let mut writes = Vec::new();
        self.writes_in_place(&mut writes);
        writes
    }

    /// Return the read and the write set. This method is exposed because it is
    /// cheaper to compute both sets  at the same time on some types.
    fn reads_and_writes(&self) -> (Vec<RRC<T>>, Vec<RRC<T>>) {
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        self.reads_and_writes_in_place(&mut reads, &mut writes);
        (reads, writes)
    }

    /// Returns all uses of ports in this type constituting both reads and writes
    fn uses(&self) -> Vec<RRC<T>> {
        let mut out = Vec::new();
        self.reads_in_place(&mut out);
        self.writes_in_place(&mut out);
        out
    }
}

impl Uses<ir::Port> for ir::Assignment {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        // Reads from the source port
        reads.push(Rc::clone(&self.src));
        // Reads from the guard ports
        reads.extend(
            self.guard
                .all_ports()
                .into_iter()
                .filter(|port| !port.borrow().is_hole()),
        );
        for port in self.guard.all_ports() {
            if !port.borrow().is_hole() {
                reads.push(Rc::clone(&port));
            }
        }
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        writes.push(Rc::clone(&self.dst));
    }
}

impl<T> Uses<ir::Port> for &[T]
where
    T: Uses<ir::Port>,
{
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        self.iter().for_each(|assign| {
            reads.extend(assign.reads());
            writes.extend(assign.writes());
        });
    }

    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        reads.extend(self.iter().flat_map(|a| a.reads()));
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        writes.extend(self.iter().flat_map(|a| a.writes()));
    }
}

impl<T> Uses<ir::Port> for Vec<T>
where
    T: Uses<ir::Port>,
{
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        self.iter().for_each(|assign| {
            reads.extend(assign.reads());
            writes.extend(assign.writes());
        });
    }

    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        reads.extend(self.iter().flat_map(|a| a.reads()));
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        writes.extend(self.iter().flat_map(|a| a.writes()));
    }
}

impl<T> Uses<ir::Port> for &T
where
    T: Uses<ir::Port>,
{
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        (*self).reads_and_writes_in_place(reads, writes)
    }

    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        (*self).reads_in_place(reads)
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        (*self).writes_in_place(writes)
    }
}

impl Uses<ir::Port> for ir::Group {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        reads.reserve(self.assignments.len());
        self.assignments.reads_in_place(reads);
        reads.push(Rc::clone(&self.done_cond));
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        writes.reserve(self.assignments.len());
        self.assignments.writes_in_place(writes)
    }

    // Manually implement read_write_sets because the one for Vec<ir::Assignment> is optimized to do only one iteration
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        self.assignments.reads_and_writes_in_place(reads, writes);
        reads.push(Rc::clone(&self.done_cond));
    }
}

impl Uses<ir::Port> for ir::CombGroup {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        reads.reserve(self.assignments.len());
        self.assignments.reads_in_place(reads);
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        writes.reserve(self.assignments.len());
        self.assignments.writes_in_place(writes)
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        self.assignments.reads_and_writes_in_place(reads, writes)
    }
}

// Implementations for control nodes
impl Uses<ir::Port> for ir::Empty {
    fn reads_in_place(&self, _: &mut Vec<RRC<ir::Port>>) {}

    fn writes_in_place(&self, _: &mut Vec<RRC<ir::Port>>) {}
}

impl Uses<ir::Port> for ir::Enable {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        self.group.borrow().reads_in_place(reads);
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        self.group.borrow().writes_in_place(writes);
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        self.group.borrow().reads_and_writes_in_place(reads, writes);
    }
}

impl Uses<ir::Port> for ir::Invoke {
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        let ir::Invoke {
            inputs,
            outputs,
            ref_cells,
            comb_group,
            ..
        } = self;
        reads.extend(inputs.iter().map(|(_, p)| p).cloned());
        writes.extend(outputs.iter().map(|(_, p)| p).cloned());
        match comb_group {
            Some(cgr) => {
                cgr.borrow().reads_and_writes_in_place(reads, writes);
            }
            None => (),
        }
        // All ports defined on the ref cells are assumed to be used.
        for (_, cell) in ref_cells {
            for port in &cell.borrow().ports {
                match &port.borrow().direction {
                    ir::Direction::Input => reads.push(Rc::clone(port)),
                    ir::Direction::Output => writes.push(Rc::clone(port)),
                    ir::Direction::Inout => unreachable!(),
                }
            }
        }
    }
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        let mut writes = Vec::new();
        self.reads_and_writes_in_place(reads, &mut writes);
    }
    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        let mut reads = Vec::new();
        self.reads_and_writes_in_place(&mut reads, writes);
    }
}

impl Uses<ir::Port> for ir::Seq {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        let stmts = &self.stmts;
        reads.reserve(stmts.len() * 2);
        stmts.iter().for_each(|stmt| stmt.reads_in_place(reads))
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        let stmts = &self.stmts;
        writes.reserve(stmts.len() * 2);
        stmts.iter().for_each(|stmt| stmt.writes_in_place(writes))
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        let stmts = &self.stmts;
        reads.reserve(stmts.len() * 2);
        writes.reserve(stmts.len() * 2);
        stmts
            .iter()
            .for_each(|stmt| stmt.reads_and_writes_in_place(reads, writes))
    }
}

impl Uses<ir::Port> for ir::Par {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        let stmts = &self.stmts;
        reads.reserve(stmts.len() * 2);
        stmts.iter().for_each(|stmt| stmt.reads_in_place(reads))
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        let stmts = &self.stmts;
        writes.reserve(stmts.len() * 2);
        stmts.iter().for_each(|stmt| stmt.writes_in_place(writes))
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        let stmts = &self.stmts;
        reads.reserve(stmts.len() * 2);
        writes.reserve(stmts.len() * 2);
        stmts
            .iter()
            .for_each(|stmt| stmt.reads_and_writes_in_place(reads, writes))
    }
}

impl Uses<ir::Port> for ir::If {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        let ir::If {
            port,
            cond,
            tbranch,
            fbranch,
            ..
        } = self;
        tbranch.reads_in_place(reads);
        fbranch.reads_in_place(reads);
        reads.push(Rc::clone(port));

        if let Some(cg) = cond {
            cg.borrow().reads_in_place(reads);
        }
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        let ir::If {
            cond,
            tbranch,
            fbranch,
            ..
        } = self;
        tbranch.writes_in_place(writes);
        fbranch.writes_in_place(writes);

        if let Some(cg) = cond {
            cg.borrow().writes_in_place(writes);
        }
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        let ir::If {
            port,
            cond,
            tbranch,
            fbranch,
            ..
        } = self;
        tbranch.reads_and_writes_in_place(reads, writes);
        fbranch.reads_and_writes_in_place(reads, writes);
        reads.push(Rc::clone(port));

        if let Some(cg) = cond {
            cg.borrow().reads_and_writes_in_place(reads, writes);
        }
    }
}

impl Uses<ir::Port> for ir::While {
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        let ir::While {
            port, cond, body, ..
        } = self;
        body.reads_in_place(reads);
        reads.push(Rc::clone(port));
        if let Some(cg) = cond {
            cg.borrow().reads_in_place(reads);
        }
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        let ir::While { cond, body, .. } = self;
        body.writes_in_place(writes);
        if let Some(cg) = cond {
            cg.borrow().writes_in_place(writes);
        }
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        let ir::While {
            port, cond, body, ..
        } = self;
        body.reads_and_writes_in_place(reads, writes);
        reads.push(Rc::clone(port));
        if let Some(cg) = cond {
            cg.borrow().reads_and_writes_in_place(reads, writes);
        }
    }
}

impl Uses<ir::Port> for ir::Control {
    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Port>>,
        writes: &mut Vec<RRC<ir::Port>>,
    ) {
        match self {
            ir::Control::Empty(em) => {
                em.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::Enable(en) => {
                en.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::Seq(seq) => {
                seq.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::Par(par) => {
                par.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::Invoke(inv) => {
                inv.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::If(if_) => {
                if_.reads_and_writes_in_place(reads, writes)
            }
            ir::Control::While(wh) => {
                wh.reads_and_writes_in_place(reads, writes)
            }
        }
    }

    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Port>>) {
        match self {
            ir::Control::Empty(em) => em.reads_in_place(reads),
            ir::Control::Enable(en) => en.reads_in_place(reads),
            ir::Control::Seq(seq) => seq.reads_in_place(reads),
            ir::Control::Par(par) => par.reads_in_place(reads),
            ir::Control::Invoke(inv) => inv.reads_in_place(reads),
            ir::Control::If(if_) => if_.reads_in_place(reads),
            ir::Control::While(wh) => wh.reads_in_place(reads),
        }
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Port>>) {
        match self {
            ir::Control::Empty(em) => em.writes_in_place(writes),
            ir::Control::Enable(en) => en.writes_in_place(writes),
            ir::Control::Seq(seq) => seq.writes_in_place(writes),
            ir::Control::Par(par) => par.writes_in_place(writes),
            ir::Control::Invoke(inv) => inv.writes_in_place(writes),
            ir::Control::If(if_) => if_.writes_in_place(writes),
            ir::Control::While(wh) => wh.writes_in_place(writes),
        }
    }
}

impl<T> Uses<ir::Cell> for T
where
    T: Uses<ir::Port>,
{
    fn reads_in_place(&self, reads: &mut Vec<RRC<ir::Cell>>) {
        let r = Uses::<ir::Port>::reads(self);
        let cells = r.iter().map(|port| port.borrow().cell_parent());
        reads.extend(cells);
    }

    fn writes_in_place(&self, writes: &mut Vec<RRC<ir::Cell>>) {
        let w = Uses::<ir::Port>::reads(self);
        let cells = w.iter().map(|port| port.borrow().cell_parent());
        writes.extend(cells);
    }

    fn reads_and_writes_in_place(
        &self,
        reads: &mut Vec<RRC<ir::Cell>>,
        writes: &mut Vec<RRC<ir::Cell>>,
    ) {
        let (p_reads, p_writes) = Uses::<ir::Port>::reads_and_writes(self);
        reads.extend(p_reads.iter().map(|p| p.borrow().cell_parent()));
        writes.extend(p_writes.iter().map(|p| p.borrow().cell_parent()));
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
    fn unique_reads_and_writes(&self) -> (Vec<RRC<T>>, Vec<RRC<T>>);
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

    fn unique_reads_and_writes(&self) -> (Vec<RRC<I>>, Vec<RRC<I>>) {
        let (reads, writes) = self.reads_and_writes();
        (
            reads
                .into_iter()
                .unique_by(|i| i.borrow().unique())
                .collect(),
            writes
                .into_iter()
                .unique_by(|i| i.borrow().unique())
                .collect(),
        )
    }
}
