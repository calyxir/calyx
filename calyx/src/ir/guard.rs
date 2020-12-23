use super::{Port, RRC};
use std::ops::{BitAnd, BitOr, Not};
use std::{cmp::Ordering, hash::Hash, rc::Rc};

/// An assignment guard which has pointers to the various ports from which it reads.
#[derive(Debug, Clone)]
pub enum Guard {
    Or(Box<Guard>, Box<Guard>),
    And(Box<Guard>, Box<Guard>),
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

impl Hash for Guard {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Guard::Or(l, r)
            | Guard::And(l, r)
            | Guard::Eq(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Geq(l, r)
            | Guard::Leq(l, r) => {
                l.hash(state);
                r.hash(state)
            }
            Guard::Not(inner) => inner.hash(state),
            Guard::Port(p) => {
                p.borrow().name.hash(state);
                p.borrow().get_parent_name().hash(state);
            }
            Guard::True => {}
        }
    }
}

/// Helper functions for the guard.
impl Guard {
    /// Mutates a guard by calling `f` on every leaf in the
    /// guard tree and replacing the leaf with the guard that `f`
    /// returns.
    pub fn for_each<F>(&mut self, f: &F)
    where
        F: Fn(RRC<Port>) -> Option<Guard>,
    {
        match self {
            Guard::Eq(l, r)
            | Guard::And(l, r)
            | Guard::Or(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Geq(l, r)
            | Guard::Leq(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Not(inner) => {
                inner.for_each(f);
            }
            Guard::Port(port) => {
                let guard = f(Rc::clone(port))
                    .unwrap_or_else(|| Guard::port(Rc::clone(port)));
                *self = guard;
            }
            Guard::True => {}
        }
    }

    /// Returns all the ports used by this guard.
    pub fn all_ports(&self) -> Vec<RRC<Port>> {
        match self {
            Guard::Port(a) => vec![Rc::clone(a)],
            Guard::And(l, r)
            | Guard::Or(l, r)
            | Guard::Eq(l, r)
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
            Guard::And(..) => "&".to_string(),
            Guard::Or(..) => "|".to_string(),
            Guard::Eq(..) => "==".to_string(),
            Guard::Neq(..) => "!=".to_string(),
            Guard::Gt(..) => ">".to_string(),
            Guard::Lt(..) => "<".to_string(),
            Guard::Geq(..) => ">=".to_string(),
            Guard::Leq(..) => "<=".to_string(),
            Guard::Not(..) => "!".to_string(),
            Guard::Port(..) | Guard::True => {
                panic!("No operator string for Guard::Port")
            }
        }
    }

    pub fn port(p: RRC<Port>) -> Self {
        if p.borrow().is_constant(1, 1) {
            Guard::True
        } else {
            Guard::Port(p)
        }
    }

    pub fn and(self, rhs: Guard) -> Self {
        if rhs == Guard::True {
            self
        } else if self == Guard::True {
            rhs
        } else if self == rhs {
            self
        } else {
            Guard::And(Box::new(self), Box::new(rhs))
        }
    }

    pub fn or(self, rhs: Guard) -> Self {
        if rhs == Guard::True || self == Guard::True {
            Guard::True
        } else if self == rhs {
            self
        } else {
            Guard::Or(Box::new(self), Box::new(rhs))
        }
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
}

/// Construct guards from ports
impl From<RRC<Port>> for Guard {
    fn from(port: RRC<Port>) -> Self {
        Guard::Port(Rc::clone(&port))
    }
}

impl PartialEq for Guard {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Guard::Or(la, ra), Guard::Or(lb, rb))
            | (Guard::And(la, ra), Guard::And(lb, rb))
            | (Guard::Eq(la, ra), Guard::Eq(lb, rb))
            | (Guard::Neq(la, ra), Guard::Neq(lb, rb))
            | (Guard::Gt(la, ra), Guard::Gt(lb, rb))
            | (Guard::Lt(la, ra), Guard::Lt(lb, rb))
            | (Guard::Geq(la, ra), Guard::Geq(lb, rb))
            | (Guard::Leq(la, ra), Guard::Leq(lb, rb)) => la == lb && ra == rb,
            (Guard::Not(a), Guard::Not(b)) => a == b,
            (Guard::Port(a), Guard::Port(b)) => {
                (a.borrow().get_parent_name(), &a.borrow().name)
                    == (b.borrow().get_parent_name(), &b.borrow().name)
            }
            (Guard::True, Guard::True) => true,
            _ => false,
        }
    }
}

impl Eq for Guard {}

/// Define order on guards
impl PartialOrd for Guard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Define an ordering on the precedence of guards. Guards are
/// considered equal when they have the same precedence.
impl Ord for Guard {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Guard::Or(..), Guard::Or(..))
            | (Guard::And(..), Guard::And(..))
            | (Guard::Eq(..), Guard::Eq(..))
            | (Guard::Neq(..), Guard::Neq(..))
            | (Guard::Gt(..), Guard::Gt(..))
            | (Guard::Lt(..), Guard::Lt(..))
            | (Guard::Geq(..), Guard::Geq(..))
            | (Guard::Leq(..), Guard::Leq(..))
            | (Guard::Not(..), Guard::Not(..))
            | (Guard::Port(..), Guard::Port(..))
            | (Guard::True, Guard::True) => Ordering::Equal,
            (Guard::Or(..), _) => Ordering::Greater,
            (_, Guard::Or(..)) => Ordering::Less,
            (Guard::And(..), _) => Ordering::Greater,
            (_, Guard::And(..)) => Ordering::Less,
            (Guard::Leq(..), _) => Ordering::Greater,
            (_, Guard::Leq(..)) => Ordering::Less,
            (Guard::Geq(..), _) => Ordering::Greater,
            (_, Guard::Geq(..)) => Ordering::Less,
            (Guard::Lt(..), _) => Ordering::Greater,
            (_, Guard::Lt(..)) => Ordering::Less,
            (Guard::Gt(..), _) => Ordering::Greater,
            (_, Guard::Gt(..)) => Ordering::Less,
            (Guard::Eq(..), _) => Ordering::Greater,
            (_, Guard::Eq(..)) => Ordering::Less,
            (Guard::Neq(..), _) => Ordering::Greater,
            (_, Guard::Neq(..)) => Ordering::Less,
            (Guard::Not(..), _) => Ordering::Greater,
            (_, Guard::Not(..)) => Ordering::Less,
            (Guard::Port(..), _) => Ordering::Greater,
            (_, Guard::Port(..)) => Ordering::Less,
        }
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
        self.and(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let or_guard = g1 | g2;
/// ```
impl BitOr for Guard {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        self.or(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let not_guard = !g1;
/// ```
impl Not for Guard {
    type Output = Self;

    fn not(self) -> Self {
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
