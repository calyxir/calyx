use crate::Printer;

use super::{NumAttr, Port, RRC};
use calyx_utils::Error;
use std::fmt::Debug;
use std::mem;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::{cmp::Ordering, hash::Hash, rc::Rc};

#[derive(Debug, Clone, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Nothing;

impl ToString for Nothing {
    fn to_string(&self) -> String {
        "".to_string()
    }
}

/// Comparison operations that can be performed between ports by [Guard::CompOp].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub enum Guard<T> {
    /// Represents `c1 || c2`.
    Or(Box<Guard<T>>, Box<Guard<T>>),
    /// Represents `c1 && c2`.
    And(Box<Guard<T>>, Box<Guard<T>>),
    /// Represents `!c1`
    Not(Box<Guard<T>>),
    #[default]
    /// The constant true
    True,
    /// Comparison operator.
    CompOp(PortComp, RRC<Port>, RRC<Port>),
    /// Uses the value on a port as the condition. Same as `p1 == true`
    Port(RRC<Port>),
    /// Other types of information.
    Info(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticTiming {
    interval: (u64, u64),
}

impl ToString for StaticTiming {
    fn to_string(&self) -> String {
        if self.interval.0 + 1 == self.interval.1 {
            format!("%{}", self.interval.0)
        } else {
            format!("%[{}:{}]", self.interval.0, self.interval.1)
        }
    }
}

impl StaticTiming {
    /// creates a new `StaticTiming` struct
    pub fn new(interval: (u64, u64)) -> Self {
        StaticTiming { interval }
    }

    /// returns the (u64, u64) interval for `struct`
    pub fn get_interval(&self) -> (u64, u64) {
        self.interval
    }

    /// overwrites the current `interval` to be `new_interval`
    pub fn set_interval(&mut self, new_interval: (u64, u64)) {
        self.interval = new_interval;
    }
}

impl<T> Hash for Guard<T>
where
    T: ToString,
{
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
            Guard::Info(i) => i.to_string().hash(state),
        }
    }
}

impl From<Guard<Nothing>> for Guard<StaticTiming> {
    /// Turns a normal guard into a static guard
    fn from(g: Guard<Nothing>) -> Self {
        match g {
            Guard::Or(left, right) => {
                let l = Self::from(*left);
                let r = Self::from(*right);
                Guard::Or(Box::new(l), Box::new(r))
            }
            Guard::And(left, right) => {
                let l = Self::from(*left);
                let r = Self::from(*right);
                Guard::And(Box::new(l), Box::new(r))
            }
            Guard::Not(c) => {
                let inside = Self::from(*c);
                Guard::Not(Box::new(inside))
            }
            Guard::True => Guard::True,
            Guard::CompOp(pc, left, right) => Guard::CompOp(pc, left, right),
            Guard::Port(p) => Guard::Port(p),
            Guard::Info(_) => {
                unreachable!(
                    "{:?}: Guard<Nothing> should not be of the
                info variant type",
                    g
                )
            }
        }
    }
}

impl<T> Guard<T> {
    /// Returns true definitely `Guard::True`.
    /// Returning false does not mean that the guard is not true.
    pub fn is_true(&self) -> bool {
        match self {
            Guard::True => true,
            Guard::Port(p) => p.borrow().is_constant(1, 1),
            _ => false,
        }
    }

    /// Checks if the guard is always false.
    /// Returning false does not mean that the guard is not false.
    pub fn is_false(&self) -> bool {
        match self {
            Guard::Not(g) => g.is_true(),
            _ => false,
        }
    }

    /// returns true if the self is !cell_name, false otherwise.
    pub fn is_not_done(&self, cell_name: &crate::Id) -> bool {
        if let Guard::Not(g) = self {
            if let Guard::Port(port) = &(**g) {
                return port.borrow().attributes.has(NumAttr::Done)
                    && port.borrow().get_parent_name() == cell_name;
            }
        }
        false
    }

    /// Update the guard in place. Replaces this guard with `upd(self)`.
    /// Uses `std::mem::take` for the in-place update.
    #[inline(always)]
    pub fn update<F>(&mut self, upd: F)
    where
        F: FnOnce(Guard<T>) -> Guard<T>,
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
            Guard::Port(..) | Guard::True | Guard::Info(_) => {
                panic!("No operator string for Guard::Port/True/Info")
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

    pub fn and(self, rhs: Guard<T>) -> Self
    where
        T: Eq,
    {
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

    pub fn or(self, rhs: Guard<T>) -> Self
    where
        T: Eq,
    {
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

    pub fn eq(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Eq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
        }
    }

    pub fn neq(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Neq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
        }
    }

    pub fn le(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Leq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
        }
    }

    pub fn lt(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Lt, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
        }
    }

    pub fn ge(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Geq, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
        }
    }

    pub fn gt(self, other: Guard<T>) -> Self
    where
        T: Debug + Eq + ToString,
    {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => {
                Guard::CompOp(PortComp::Gt, l, r)
            }
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Eq using `{}' and `{}'",
                    Printer::guard_str(&l),
                    Printer::guard_str(&r),
                )
            }
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
            Guard::Info(_) => vec![],
        }
    }
}

/// Helper functions for the guard.
impl<T> Guard<T> {
    /// Mutates a guard by calling `f` on every leaf in the
    /// guard tree and replacing the leaf with the guard that `f`
    /// returns.
    pub fn for_each<F>(&mut self, f: &mut F)
    where
        F: FnMut(RRC<Port>) -> Option<Guard<T>>,
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
            Guard::Info(_) =>
                // Info shouldn't count as port
                {}
        }
    }

    /// runs f(info) on each Guard::Info in `guard`.
    /// if `f(info)` = Some(result)` replaces interval with result.
    /// if `f(info)` = None` does nothing.
    pub fn for_each_info<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut T) -> Option<Guard<T>>,
    {
        match self {
            Guard::And(l, r) | Guard::Or(l, r) => {
                l.for_each_info(f);
                r.for_each_info(f);
            }
            Guard::Not(inner) => {
                inner.for_each_info(f);
            }
            Guard::True | Guard::Port(_) | Guard::CompOp(_, _, _) => {}
            Guard::Info(timing_interval) => {
                if let Some(new_interval) = f(timing_interval) {
                    *self = new_interval
                }
            }
        }
    }

    /// runs f(info) on each info in `guard`.
    /// f should return Result<(), Error>, meaning that it essentially does
    /// nothing if the `f` returns OK(()), but returns an appropraite error otherwise
    pub fn check_for_each_info<F>(&self, f: &mut F) -> Result<(), Error>
    where
        F: Fn(&T) -> Result<(), Error>,
    {
        match self {
            Guard::And(l, r) | Guard::Or(l, r) => {
                let l_result = l.check_for_each_info(f);
                if l_result.is_err() {
                    l_result
                } else {
                    r.check_for_each_info(f)
                }
            }
            Guard::Not(inner) => inner.check_for_each_info(f),
            Guard::True | Guard::Port(_) | Guard::CompOp(_, _, _) => Ok(()),
            Guard::Info(timing_interval) => f(timing_interval),
        }
    }
}

impl Guard<StaticTiming> {
    /// updates self -> self & interval
    pub fn add_interval(&mut self, timing_interval: StaticTiming) {
        self.update(|g| g.and(Guard::Info(timing_interval)));
    }
}

/// Construct guards from ports
impl<T> From<RRC<Port>> for Guard<T> {
    fn from(port: RRC<Port>) -> Self {
        Guard::Port(Rc::clone(&port))
    }
}

impl<T> PartialEq for Guard<T>
where
    T: Eq,
{
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
            (Guard::Info(i1), Guard::Info(i2)) => i1 == i2,
            _ => false,
        }
    }
}

impl<T> Eq for Guard<T> where T: Eq {}

/// Define order on guards
impl<T> PartialOrd for Guard<T>
where
    T: Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Define an ordering on the precedence of guards. Guards are
/// considered equal when they have the same precedence.
impl<T> Ord for Guard<T>
where
    T: Eq,
{
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Guard::Or(..), Guard::Or(..))
            | (Guard::And(..), Guard::And(..))
            | (Guard::CompOp(..), Guard::CompOp(..))
            | (Guard::Not(..), Guard::Not(..))
            | (Guard::Port(..), Guard::Port(..))
            | (Guard::Info(_), Guard::Info(_))
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
            // maybe we should change this?
            (Guard::Info(..), _) => Ordering::Greater,
            (_, Guard::Info(..)) => Ordering::Less,
        }
    }
}

/////////////// Sugar for convience constructors /////////////

/// Construct a Guard::And:
/// ```
/// let and_guard = g1 & g2;
/// ```
impl<T> BitAnd for Guard<T>
where
    T: Eq,
{
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        self.and(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let or_guard = g1 | g2;
/// ```
impl<T> BitOr for Guard<T>
where
    T: Eq,
{
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        self.or(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let not_guard = !g1;
/// ```
impl<T> Not for Guard<T> {
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
impl<T> BitOrAssign for Guard<T>
where
    T: Eq,
{
    fn bitor_assign(&mut self, other: Self) {
        self.update(|old| old | other)
    }
}

/// Update a Guard with Or.
/// ```
/// g1 &= g2;
/// ```
impl<T> BitAndAssign for Guard<T>
where
    T: Eq,
{
    fn bitand_assign(&mut self, other: Self) {
        self.update(|old| old & other)
    }
}
