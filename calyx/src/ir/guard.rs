use super::{Port, RRC};
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
}

/// Helper functions for the guard.
impl Guard {
    pub fn and_vec(&self, guards: &mut Vec<Guard>) -> Self {
        if let Guard::And(inner) = &self {
            let mut new: Vec<_> = inner.clone();
            new.append(guards);
            Guard::And(new)
        } else {
            let mut new: Vec<Guard> = vec![self.clone()];
            new.append(guards);
            Guard::And(new)
        }
    }

    pub fn and(&self, rhs: Guard) -> Self {
        self.and_vec(&mut vec![rhs])
    }
    // TODO(rachit): Implement a tree walking function.
    // fn for_each
    pub fn for_each<F>(&mut self, f: &F)
    where
        F: Fn(RRC<Port>) -> Guard,
    {
        match self {
            Guard::And(ands) => {
                ands.iter_mut().for_each(|guard| guard.for_each(f))
            }
            Guard::Or(ors) => {
                ors.iter_mut().for_each(|guard| guard.for_each(f))
            }
            // Guard::Eq(l, r) => {
            //     Guard::Eq(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Neq(l, r) => {
            //     Guard::Neq(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Gt(l, r) => {
            //     Guard::Gt(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Lt(l, r) => {
            //     Guard::Lt(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Geq(l, r) => {
            //     Guard::Geq(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Leq(l, r) => {
            //     Guard::Leq(Box::new(l.for_each(f)), Box::new(r.for_each(f)))
            // }
            // Guard::Not(inner) => Guard::Not(Box::new(inner.for_each(f))),
            Guard::Port(port) => *self = f(Rc::clone(port)),
            _ => unimplemented!(),
        }
    }
}
