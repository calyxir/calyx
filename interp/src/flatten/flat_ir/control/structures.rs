use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::attributes::Attributes, structures::index_trait::impl_index,
};

impl_index!(pub ControlIdx);

pub type CtrlVec = SmallVec<[ControlIdx; 4]>;

#[derive(Debug)]
pub struct Empty {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct Enable {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct Seq {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct Par {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct If {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct While {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub struct Invoke {
    pub attributes: Attributes,
}

#[derive(Debug)]
pub enum ControlNode {}
