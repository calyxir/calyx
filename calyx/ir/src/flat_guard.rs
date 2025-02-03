use crate::Nothing;

use super::guard::{Guard, PortComp};
use super::{Port, RRC};

#[derive(Debug, Copy, Clone)]
pub struct GuardRef(u32);

impl GuardRef {
    /// Check whether this refers to a `FlatGuard::True`. (We can do this because the first guard
    /// in the pool is always `True`.)
    pub fn is_true(&self) -> bool {
        self.0 == 0
    }

    /// Get the underlying number for this reference. Clients should only rely on this being unique
    /// for non-equal guards in a single pool; no other aspects of the number are relevant.
    pub fn index(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum FlatGuard {
    Or(GuardRef, GuardRef),
    And(GuardRef, GuardRef),
    Not(GuardRef),
    True,
    CompOp(PortComp, RRC<Port>, RRC<Port>),
    Port(RRC<Port>),
}

impl FlatGuard {
    pub fn is_true(&self) -> bool {
        match self {
            FlatGuard::True => true,
            FlatGuard::Port(p) => p.borrow().is_constant(1, 1),
            _ => false,
        }
    }
}

/// A `GuardPool` is an "arena"-style storage area for `FlatGuard`s.
///
/// Some invariants for the underlying vector:
/// * `GuardRefs` are always within the same pool (obviously).
/// * The underlyings numbers in `GuardRef`s can only go "backward," in the sense that
///   they refer to smaller indices than the current `FlatGuard`.
/// * The first `FlatGuard` is always `FlatGuard::True`.
///
/// This could be used to do some interesting hash-consing/deduplication; it currently does the
/// weakest possible form of that: deduplicating `True` guards only.
pub struct GuardPool(Vec<FlatGuard>);

impl GuardPool {
    pub fn new() -> Self {
        let mut vec = Vec::<FlatGuard>::with_capacity(1024);
        vec.push(FlatGuard::True);
        Self(vec)
    }

    fn add(&mut self, guard: FlatGuard) -> GuardRef {
        // `True` is always the first guard.
        if guard.is_true() {
            return GuardRef(0);
        }

        self.0.push(guard);
        GuardRef(
            (self.0.len() - 1)
                .try_into()
                .expect("too many guards in the pool"),
        )
    }

    pub fn flatten(&mut self, old: &Guard<Nothing>) -> GuardRef {
        match old {
            Guard::Or(l, r) => {
                let flat_l = self.flatten(l);
                let flat_r = self.flatten(r);
                self.add(FlatGuard::Or(flat_l, flat_r))
            }
            Guard::And(l, r) => {
                let flat_l = self.flatten(l);
                let flat_r = self.flatten(r);
                self.add(FlatGuard::And(flat_l, flat_r))
            }
            Guard::Not(g) => {
                let flat_g = self.flatten(g);
                self.add(FlatGuard::Not(flat_g))
            }
            Guard::True => self.add(FlatGuard::True),
            Guard::CompOp(op, l, r) => {
                self.add(FlatGuard::CompOp(op.clone(), l.clone(), r.clone()))
            }
            Guard::Port(p) => self.add(FlatGuard::Port(p.clone())),
            Guard::Info(_) => {
                panic!("flat guard sees info, think about this more")
            }
        }
    }

    pub fn get(&self, guard: GuardRef) -> &FlatGuard {
        &self.0[guard.0 as usize]
    }

    #[cfg(debug_assertions)]
    pub fn display(&self, guard: &FlatGuard) -> String {
        match guard {
            FlatGuard::Or(l, r) => format!(
                "({} | {})",
                self.display(self.get(*l)),
                self.display(self.get(*r))
            ),
            FlatGuard::And(l, r) => format!(
                "({} & {})",
                self.display(self.get(*l)),
                self.display(self.get(*r))
            ),
            FlatGuard::Not(g) => format!("!{}", self.display(self.get(*g))),
            FlatGuard::True => "true".to_string(),
            FlatGuard::CompOp(op, l, r) => {
                let op_str = match op {
                    PortComp::Eq => "==",
                    PortComp::Neq => "!=",
                    PortComp::Lt => "<",
                    PortComp::Leq => "<=",
                    PortComp::Gt => ">",
                    PortComp::Geq => ">=",
                };
                format!(
                    "({} {} {})",
                    l.borrow().canonical(),
                    op_str,
                    r.borrow().canonical()
                )
            }
            FlatGuard::Port(p) => format!("{}", p.borrow().canonical()),
        }
    }

    /// Iterate over *all* the guards in the pool.
    pub fn iter(&self) -> impl Iterator<Item = (GuardRef, &FlatGuard)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, g)| (GuardRef(i.try_into().unwrap()), g))
    }
}

impl Default for GuardPool {
    fn default() -> Self {
        Self::new()
    }
}
