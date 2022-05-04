use super::{Port, RRC};
use std::mem;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::{cmp::Ordering, hash::Hash, rc::Rc};

/// Comparison operations that can be performed between ports by [Guard::CompOp].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortComp {
    /// p1 == p2
    Eq,
    /// p1 != p2
    Neq,
    /// p1 > p2
    Gt,
    /// p1 < p2
    Lt,
    /// p1 >= p2
    Geq,
    /// p1 <= p2
    Leq,
}

/// An assignment guard which has pointers to the various ports from which it reads.
#[derive(Debug, Clone)]
pub enum Guard {
    /// Represents `c1 || c2`.
    Or(Box<Guard>, Box<Guard>),
    /// Represents `c1 && c2`.
    And(Box<Guard>, Box<Guard>),
    /// Represents `!c1`
    Not(Box<Guard>),
    /// The constant true
    True,
    /// Comparison operator.
    CompOp(PortComp, RRC<Port>, RRC<Port>),
    /// Uses the value on a port as the condition. Same as `p1 == true`
    Port(RRC<Port>),
}

impl Default for Guard {
    fn default() -> Self {
        Guard::True
    }
}

impl Hash for Guard {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Guard::Or(l, r) | Guard::And(l, r) => {
                l.hash(state);
                r.hash(state)
            }
            Guard::CompOp(_, l, r) => {
                l.borrow().name.hash(state);
                l.borrow().get_parent_name().hash(state);
                r.borrow().name.hash(state);
                r.borrow().get_parent_name().hash(state);
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
    pub fn for_each<F>(&mut self, f: &mut F)
    where
        F: FnMut(RRC<Port>) -> Option<Guard>,
    {
        match self {
            Guard::And(l, r) | Guard::Or(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Not(inner) => {
                inner.for_each(f);
            }
            Guard::CompOp(_, l, r) => {
                match f(Rc::clone(l)) {
                    Some(Guard::Port(p)) => *l = p,
                    Some(_) => unreachable!(
                        "Cannot replace port inside comparison operator"
                    ),
                    None => {}
                }
                match f(Rc::clone(r)) {
                    Some(Guard::Port(p)) => *r = p,
                    Some(_) => unreachable!(
                        "Cannot replace port inside comparison operator"
                    ),
                    None => {}
                }
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
            Guard::And(l, r) | Guard::Or(l, r) => {
                let mut atoms = l.all_ports();
                atoms.append(&mut r.all_ports());
                atoms
            }
            Guard::CompOp(_, l, r) => {
                vec![Rc::clone(l), Rc::clone(r)]
            }
            Guard::Not(g) => g.all_ports(),
            Guard::True => vec![],
        }
    }

    /// Returns true if this is a `Guard::True`.
    pub fn is_true(&self) -> bool {
        match self {
            Guard::True => true,
            Guard::Port(p) => p.borrow().is_constant(1, 1),
            _ => false,
        }
    }

    /// Update the guard in place. Replaces this guard with `upd(self)`.
    /// Uses `std::mem::take` for the in-place update.
    #[inline(always)]
    pub fn update<F>(&mut self, upd: F)
    where
        F: FnOnce(Guard) -> Guard,
    {
        let old = mem::take(self);
        let new = upd(old);
        *self = new;
    }

    /// Return the string corresponding to the guard operation.
    pub fn op_str(&self) -> String {
        match self {
            Guard::And(..) => "&".to_string(),
            Guard::Or(..) => "|".to_string(),
            Guard::CompOp(op, _, _) => match op {
                PortComp::Eq => "==".to_string(),
                PortComp::Neq => "!=".to_string(),
                PortComp::Gt => ">".to_string(),
                PortComp::Lt => "<".to_string(),
                PortComp::Geq => ">=".to_string(),
                PortComp::Leq => "<=".to_string(),
            },
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
        match (self, rhs) {
            (Guard::True, _) | (_, Guard::True) => Guard::True,
            (Guard::Not(n), g) | (g, Guard::Not(n)) => {
                if *n == Guard::True {
                    g
                } else {
                    Guard::Or(Box::new(Guard::Not(n)), Box::new(g))
                }
            }
            (l, r) => {
                if l == r {
                    l
                } else {
                    Guard::Or(Box::new(l), Box::new(r))
                }
            }
        }
    }

    pub fn eq(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Eq, l, r)
            }
            (l, r) => {
                unreachable!("Cannot build Guard::Eq using {:?} and {:?}", l, r)
            }
        }
    }

    pub fn neq(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Neq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Neq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn le(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Leq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Leq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn lt(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Lt, l, r)
            }
            (l, r) => {
                unreachable!("Cannot build Guard::Lt using {:?} and {:?}", l, r)
            }
        }
    }

    pub fn ge(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Geq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Geq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn gt(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Gt, l, r)
            }
            (l, r) => {
                unreachable!("Cannot build Guard::Gt using {:?} and {:?}", l, r)
            }
        }
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
            | (Guard::And(la, ra), Guard::And(lb, rb)) => la == lb && ra == rb,
            (Guard::CompOp(opa, la, ra), Guard::CompOp(opb, lb, rb)) => {
                (opa == opb)
                    && (la.borrow().get_parent_name(), &la.borrow().name)
                        == (lb.borrow().get_parent_name(), &lb.borrow().name)
                    && (ra.borrow().get_parent_name(), &ra.borrow().name)
                        == (rb.borrow().get_parent_name(), &rb.borrow().name)
            }
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
            | (Guard::CompOp(..), Guard::CompOp(..))
            | (Guard::Not(..), Guard::Not(..))
            | (Guard::Port(..), Guard::Port(..))
            | (Guard::True, Guard::True) => Ordering::Equal,
            (Guard::Or(..), _) => Ordering::Greater,
            (_, Guard::Or(..)) => Ordering::Less,
            (Guard::And(..), _) => Ordering::Greater,
            (_, Guard::And(..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Leq, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Leq, ..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Geq, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Geq, ..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Lt, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Lt, ..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Gt, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Gt, ..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Eq, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Eq, ..)) => Ordering::Less,
            (Guard::CompOp(PortComp::Neq, ..), _) => Ordering::Greater,
            (_, Guard::CompOp(PortComp::Neq, ..)) => Ordering::Less,
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
            Guard::CompOp(PortComp::Eq, lhs, rhs) => {
                Guard::CompOp(PortComp::Neq, lhs, rhs)
            }
            Guard::CompOp(PortComp::Neq, lhs, rhs) => {
                Guard::CompOp(PortComp::Eq, lhs, rhs)
            }
            Guard::CompOp(PortComp::Gt, lhs, rhs) => {
                Guard::CompOp(PortComp::Leq, lhs, rhs)
            }
            Guard::CompOp(PortComp::Lt, lhs, rhs) => {
                Guard::CompOp(PortComp::Geq, lhs, rhs)
            }
            Guard::CompOp(PortComp::Geq, lhs, rhs) => {
                Guard::CompOp(PortComp::Lt, lhs, rhs)
            }
            Guard::CompOp(PortComp::Leq, lhs, rhs) => {
                Guard::CompOp(PortComp::Gt, lhs, rhs)
            }
            Guard::Not(expr) => *expr,
            _ => Guard::Not(Box::new(self)),
        }
    }
}

/// Update a Guard with Or.
/// ```
/// g1 |= g2;
/// ```
impl BitOrAssign for Guard {
    fn bitor_assign(&mut self, other: Self) {
        self.update(|old| old | other)
    }
}

/// Update a Guard with Or.
/// ```
/// g1 &= g2;
/// ```
impl BitAndAssign for Guard {
    fn bitand_assign(&mut self, other: Self) {
        self.update(|old| old & other)
    }
}
