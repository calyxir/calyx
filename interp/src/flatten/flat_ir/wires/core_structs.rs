use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::environment::{LocalCellRef, PortRef},
};

use super::guards::GuardIdx;

#[derive(Debug)]
pub struct Assignment {
    pub dst: PortRef,
    pub src: PortRef,
    pub guard: GuardIdx,
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct Group {
    name: Identifier,

    pub assignments: Vec<Assignment>,

    pub holes: SmallVec<[LocalCellRef; 3]>,

    pub attributes: Attributes,
}
