use super::{Port, RRC};
use std::ops::{BitAnd, BitOr, Not};
use std::rc::Rc;

/// An assignment guard which has pointers to the various ports from which it reads.
#[derive(Debug, Clone)]
pub enum Guard {
    And(Vec<Guard>),
    Or(Vec<Guard>),
    Eq(Box<Guard>, Box<Guard>),
    Neq(Box<Guard>, Box<Guard>),
    Gt(Box<Guard>, Box<Guard>),
    Lt(Box<Guard>, Box<Guard>),
    Geq(Box<Guard>, Box<Guard>),
    Leq(Box<Guard>, Box<Guard>),
    Not(Box<Guard>),
    Port(RRC<Port>),
    True,
}

/// Helper functions for the guard.
impl Guard {
    /// Mutates a guard by calling `f` on every leaf in the
    /// guard tree and replacing the leaf with the guard that `f`
    /// returns.
    pub fn for_each<F>(&mut self, f: &F)
    where
        F: Fn(&Port) -> Option<Guard>,
    {
        match self {
            Guard::And(ands) => {
                ands.iter_mut().for_each(|guard| guard.for_each(f))
            }
            Guard::Or(ors) => {
                ors.iter_mut().for_each(|guard| guard.for_each(f))
            }
            Guard::Eq(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Neq(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Gt(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Lt(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Geq(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Leq(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Not(inner) => {
                inner.for_each(f);
            }
            Guard::Port(port) => {
                let guard =
                    f(&port.borrow()).unwrap_or(Guard::Port(Rc::clone(port)));
                *self = guard;
            }
            Guard::True => {}
        }
    }

    /// Returns all the ports used by this guard.
    pub fn all_ports(&self) -> Vec<RRC<Port>> {
        match self {
            Guard::Port(a) => vec![Rc::clone(a)],
            Guard::Or(gs) | Guard::And(gs) => {
                gs.iter().map(|g| g.all_ports()).flatten().collect()
            }
            Guard::Eq(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Leq(l, r)
            | Guard::Geq(l, r) => {
                let mut atoms = l.all_ports();
                atoms.append(&mut r.all_ports());
                atoms
            }
            Guard::Not(g) => g.all_ports(),
            Guard::True => vec![],
        }
    }

    /// Return the string corresponding to the guard operation.
    pub fn op_str(&self) -> String {
        match self {
            Guard::And(_) => "&".to_string(),
            Guard::Or(_) => "|".to_string(),
            Guard::Eq(_, _) => "==".to_string(),
            Guard::Neq(_, _) => "!=".to_string(),
            Guard::Gt(_, _) => ">".to_string(),
            Guard::Lt(_, _) => "<".to_string(),
            Guard::Geq(_, _) => ">=".to_string(),
            Guard::Leq(_, _) => "<=".to_string(),
            Guard::Not(_) => "!".to_string(),
            Guard::Port(_) | Guard::True => {
                panic!("No operator string for Guard::Port")
            }
        }
    }

    ////////////// Convinience constructors ///////////////////
    pub fn and_vec(&self, guards: &mut Vec<Guard>) -> Self {
        let mut new;
        if let Guard::And(inner) = &self {
            new = inner.clone();
            new.append(guards);
        } else {
            new = vec![self.clone()];
            new.append(guards);
        }
        // filter out redundant guards
        // XXX fix this filter. it was filtering out too much
        // new.retain(|guard| {
        //     if !matches!(guard, Guard::True) {
        //         return true;
        //     }

        //     println!("c {:?}", guard);
        //     if let Guard::Port(p) = guard {
        //         if p.borrow().is_constant(1, 1) {
        //             println!("p {:?}", p.borrow());
        //             return true;
        //         }
        //     }
        //     true
        // });
        Guard::And(new)
    }

    pub fn and(&self, rhs: Guard) -> Self {
        self.and_vec(&mut vec![rhs])
    }

    pub fn or(self, other: Guard) -> Self {
        Guard::Or(vec![self, other])
    }

    pub fn eq(self, other: Guard) -> Self {
        Guard::Eq(Box::new(self), Box::new(other))
    }

    pub fn neq(self, other: Guard) -> Self {
        Guard::Neq(Box::new(self), Box::new(other))
    }

    pub fn le(self, other: Guard) -> Self {
        Guard::Leq(Box::new(self), Box::new(other))
    }

    pub fn lt(self, other: Guard) -> Self {
        Guard::Lt(Box::new(self), Box::new(other))
    }

    pub fn ge(self, other: Guard) -> Self {
        Guard::Geq(Box::new(self), Box::new(other))
    }

    pub fn gt(self, other: Guard) -> Self {
        Guard::Gt(Box::new(self), Box::new(other))
    }

    pub fn not(self) -> Self {
        match self {
            Guard::Eq(lhs, rhs) => Guard::Neq(lhs, rhs),
            Guard::Neq(lhs, rhs) => Guard::Eq(lhs, rhs),
            Guard::Gt(lhs, rhs) => Guard::Leq(lhs, rhs),
            Guard::Lt(lhs, rhs) => Guard::Geq(lhs, rhs),
            Guard::Geq(lhs, rhs) => Guard::Lt(lhs, rhs),
            Guard::Leq(lhs, rhs) => Guard::Gt(lhs, rhs),
            Guard::Not(expr) => *expr,
            _ => Guard::Not(Box::new(self)),
        }
    }
}

/// Construct guards from ports
impl From<RRC<Port>> for Guard {
    fn from(port: RRC<Port>) -> Self {
        Guard::Port(Rc::clone(&port))
    }
}

/////////////// Sugar for convience constructors /////////////

/// Construct a Guard::And:
/// ```
/// let and_guard = g1 & g2;
/// ```
impl BitAnd for Guard {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        Guard::And(vec![self, other])
    }
}

/// Construct a Guard::Or:
/// ```
/// let or_guard = g1 | g2;
/// ```
impl BitOr for Guard {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        Guard::Or(vec![self, other])
    }
}

/// Construct a Guard::Or:
/// ```
/// let not_guard = !g1;
/// ```
impl Not for Guard {
    type Output = Self;

    fn not(self) -> Self {
        self.not()
    }
}
