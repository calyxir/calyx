use super::{Port, RRC};
use super::guard::{PortComp, Guard};

#[derive(Debug, Copy, Clone)]
pub struct GuardRef(u32);

impl std::fmt::Display for GuardRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

/// A GuardPool stores FlatGuard. It can have multiple "roots," or it can have just one (as in when
/// we are just replacing a single guard with this). It has an important invariant: GuardRefs are
/// always within the same pool (obviously), and they can only go "backward," in the sense that
/// they refer to smaller indices than the current FlatGuard.
/// 
/// This currently does not do *any* kind of hash-consing or deduplication. It also naively grows
/// the underlying vector; a more efficient implementation would pre-allocate the space. But we are
/// currently focused on building up guards of unknown size, so I'm leaving this off for now.
pub struct GuardPool(Vec<FlatGuard>);

impl GuardPool {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, guard: FlatGuard) -> GuardRef {
        self.0.push(guard);
        GuardRef((self.0.len() - 1).try_into().expect("too many guards in the pool"))
    }
    
    pub fn flatten(&mut self, old: &Guard) -> GuardRef {
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
            Guard::CompOp(op, l, r) => self.add(FlatGuard::CompOp(op.clone(), l.clone(), r.clone())),
            Guard::Port(p) => self.add(FlatGuard::Port(p.clone())),
        }
    }

    pub fn get(&self, guard: GuardRef) -> &FlatGuard {
        &self.0[guard.0 as usize]
    }

    #[cfg(debug_assertions)]
    pub fn display(&self, guard: &FlatGuard) -> String {
        match guard {
            FlatGuard::Or(l, r) => format!("({} | {})", self.display(self.get(*l)), self.display(self.get(*r))),
            FlatGuard::And(l, r) => format!("({} & {})", self.display(self.get(*l)), self.display(self.get(*r))),
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
                format!("({} {} {})", l.borrow().canonical(), op_str, r.borrow().canonical())
            },
            FlatGuard::Port(p) => format!("{}", p.borrow().canonical()),
        }
    }

    /// Iterate over *all* the guards in the pool.
    pub fn iter(&self) -> impl Iterator<Item = (GuardRef, &FlatGuard)> {
        self.0.iter().enumerate().map(|(i, g)| {
            (GuardRef(i.try_into().unwrap()), g)
        })
    }
}
