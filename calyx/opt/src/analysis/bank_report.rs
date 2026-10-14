use calyx_ir as ir;
use std::fmt;

/// Whether a group reads from or writes to a memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccessKind {
    /// The group drives an address (or content-enable) port without write-enable.
    Read,
    /// The group also drives the memory's write-enable port.
    Write,
}

impl fmt::Display for AccessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessKind::Read => write!(f, "read"),
            AccessKind::Write => write!(f, "write"),
        }
    }
}

/// A group that accesses a memory, together with the kind of access.
/// Sites are ordered by group name and then by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessSite {
    /// The group (or combinational group) performing the access.
    pub group: ir::Id,
    /// Whether the access is a read or a write.
    pub kind: AccessKind,
}

impl AccessSite {
    pub(crate) fn new(group: ir::Id, kind: AccessKind) -> Self {
        AccessSite { group, kind }
    }

    fn order_key(&self) -> (String, AccessKind) {
        (self.group.to_string(), self.kind)
    }
}

impl PartialOrd for AccessSite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccessSite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order_key().cmp(&other.order_key())
    }
}

/// A pair of accesses to one memory that can happen in the same cycle.
/// `left` is the smaller of the two sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryConflict {
    /// The memory both sites access.
    pub memory: ir::Id,
    /// The smaller site of the pair.
    pub left: AccessSite,
    /// The larger site of the pair.
    pub right: AccessSite,
}

impl MemoryConflict {
    pub(crate) fn new(memory: ir::Id, a: AccessSite, b: AccessSite) -> Self {
        let (left, right) = if a <= b { (a, b) } else { (b, a) };
        MemoryConflict {
            memory,
            left,
            right,
        }
    }

    fn order_key(&self) -> (String, AccessSite, AccessSite) {
        (self.memory.to_string(), self.left, self.right)
    }
}

/// The minimum number of banks a memory needs to serve all of its
/// simultaneous accesses without a conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBanks {
    /// The memory this entry describes.
    pub memory: ir::Id,
    /// The size of the largest set of pairwise-simultaneous accesses, and at
    /// least 1.
    pub min_banks: u64,
}

/// The kind of a loop-carried dependence between two accesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DepKind {
    /// A write followed by a read.
    Raw,
    /// A read followed by a write.
    War,
    /// A write followed by a write.
    Waw,
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DepKind::Raw => write!(f, "raw"),
            DepKind::War => write!(f, "war"),
            DepKind::Waw => write!(f, "waw"),
        }
    }
}

/// A dependence between two accesses to one memory that carries across
/// iterations of a loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDependence {
    /// The memory both sites access.
    pub memory: ir::Id,
    /// The access in the earlier iteration.
    pub from: AccessSite,
    /// The access in the later iteration.
    pub to: AccessSite,
    /// Whether this is a read-after-write, write-after-read or
    /// write-after-write dependence.
    pub kind: DepKind,
}

impl MemoryDependence {
    fn order_key(&self) -> (String, AccessSite, AccessSite, DepKind) {
        (self.memory.to_string(), self.from, self.to, self.kind)
    }
}

/// The bank a single access is assigned to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankAssignment {
    /// The memory being accessed.
    pub memory: ir::Id,
    /// The access that is assigned a bank.
    pub site: AccessSite,
    /// The bank number, counted from 0.
    pub bank: u64,
}

impl BankAssignment {
    fn order_key(&self) -> (String, AccessSite) {
        (self.memory.to_string(), self.site)
    }
}

/// The result of [`BankConflictAnalysis`](super::BankConflictAnalysis) on one
/// component. Every vector is sorted by its fields from left to right and has
/// no duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BankReport {
    /// Pairs of simultaneous accesses to the same memory.
    pub conflicts: Vec<MemoryConflict>,
    /// The minimum bank count of every memory, including unused ones.
    pub banks: Vec<MemoryBanks>,
    /// A bank for every access, assigned greedily in site order.
    pub assignments: Vec<BankAssignment>,
    /// Dependences carried across loop iterations.
    pub dependences: Vec<MemoryDependence>,
}

impl BankReport {
    pub(crate) fn from_parts(
        mut conflicts: Vec<MemoryConflict>,
        mut banks: Vec<MemoryBanks>,
        mut assignments: Vec<BankAssignment>,
        mut dependences: Vec<MemoryDependence>,
    ) -> Self {
        conflicts.sort_by_key(|a| a.order_key());
        conflicts.dedup();
        banks.sort_by(|a, b| a.memory.to_string().cmp(&b.memory.to_string()));
        assignments.sort_by_key(|a| a.order_key());
        dependences.sort_by_key(|a| a.order_key());
        dependences.dedup();
        BankReport {
            conflicts,
            banks,
            assignments,
            dependences,
        }
    }

    /// Whether `memory` has a loop-carried dependence of `kind` from group
    /// `from` to group `to`.
    pub fn has_dependence(
        &self,
        memory: &str,
        from: &str,
        to: &str,
        kind: DepKind,
    ) -> bool {
        self.dependences.iter().any(|d| {
            d.memory == memory
                && d.from.group == from
                && d.to.group == to
                && d.kind == kind
        })
    }

    /// The number of loop-carried dependences on `memory`.
    pub fn dependence_count(&self, memory: &str) -> usize {
        self.dependences
            .iter()
            .filter(|d| d.memory == memory)
            .count()
    }

    /// The bank assigned to the `kind` access of `group` on `memory`, if any.
    pub fn bank_of(
        &self,
        memory: &str,
        group: &str,
        kind: AccessKind,
    ) -> Option<u64> {
        self.assignments
            .iter()
            .find(|a| {
                a.memory == memory
                    && a.site.group == group
                    && a.site.kind == kind
            })
            .map(|a| a.bank)
    }

    /// The number of distinct banks used by the accesses to `memory`.
    pub fn distinct_banks(&self, memory: &str) -> u64 {
        let mut seen = std::collections::BTreeSet::new();
        for a in &self.assignments {
            if a.memory == memory {
                seen.insert(a.bank);
            }
        }
        seen.len() as u64
    }

    /// The minimum number of banks needed by `memory`, or `None` if the
    /// component has no such memory.
    pub fn min_banks(&self, memory: &str) -> Option<u64> {
        self.banks
            .iter()
            .find(|b| b.memory == memory)
            .map(|b| b.min_banks)
    }

    /// Whether any two accesses to the same memory can share a cycle.
    pub fn has_conflict(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

impl fmt::Display for BankReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in &self.conflicts {
            writeln!(
                f,
                "conflict {} {} {} {} {}",
                c.memory,
                c.left.group,
                c.left.kind,
                c.right.group,
                c.right.kind
            )?;
        }
        for b in &self.banks {
            writeln!(f, "banks {} {}", b.memory, b.min_banks)?;
        }
        for a in &self.assignments {
            writeln!(
                f,
                "assign {} {} {} {}",
                a.memory, a.site.group, a.site.kind, a.bank
            )?;
        }
        for d in &self.dependences {
            writeln!(
                f,
                "dep {} {} {} {} {} {}",
                d.memory,
                d.kind,
                d.from.group,
                d.from.kind,
                d.to.group,
                d.to.kind
            )?;
        }
        Ok(())
    }
}
