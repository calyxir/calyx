use super::{Port, RRC};
use std::rc::Rc;

/// An assignment guard which has pointers to the various ports from which it reads.
#[derive(Debug)]
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
            Guard::Port(_) => panic!("No operator string for Guard::Port"),
        }
    }
}
