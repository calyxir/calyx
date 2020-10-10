use super::{Port, RRC};

/// An assignment guard which has pointers to the various ports from which it reads.
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
}

/// Helper functions for the guard.
impl Guard {
    // TODO(rachit): Implement a tree walking function.
    // fn for_each
}
