use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::wires::core_structs::{CombGroupIdx, GroupIdx},
    structures::{
        environment::PortRef, index_trait::impl_index, indexed_map::IndexedMap,
    },
};

impl_index!(pub ControlIdx);
pub type ControlMap = IndexedMap<ControlNode, ControlIdx>;

pub type CtrlVec = SmallVec<[ControlIdx; 4]>;

#[derive(Debug)]
pub struct Empty;

#[derive(Debug)]
pub struct Enable(GroupIdx);

impl Enable {
    pub fn group(&self) -> GroupIdx {
        self.0
    }
}

#[derive(Debug)]
pub struct Seq(CtrlVec);

impl Seq {
    pub fn stms(&self) -> &[ControlIdx] {
        &self.0
    }
}

#[derive(Debug)]
pub struct Par(CtrlVec);

impl Par {
    pub fn stms(&self) -> &[ControlIdx] {
        &self.0
    }
}

#[derive(Debug)]
pub struct If {
    cond_port: PortRef,
    cond_group: Option<CombGroupIdx>,
    tbranch: ControlIdx,
    fbranch: ControlIdx,
}

impl If {
    pub fn cond_port(&self) -> PortRef {
        self.cond_port
    }

    pub fn cond_group(&self) -> Option<CombGroupIdx> {
        self.cond_group
    }

    pub fn tbranch(&self) -> ControlIdx {
        self.tbranch
    }

    pub fn fbranch(&self) -> ControlIdx {
        self.fbranch
    }
}

#[derive(Debug)]
pub struct While {
    cond_port: PortRef,
    cond_group: Option<CombGroupIdx>,
    body: ControlIdx,
}

impl While {
    pub fn cond_port(&self) -> PortRef {
        self.cond_port
    }

    pub fn cond_group(&self) -> Option<CombGroupIdx> {
        self.cond_group
    }

    pub fn body(&self) -> ControlIdx {
        self.body
    }
}

#[derive(Debug)]
pub struct Invoke {
    // TODO: add invoke stuff
}

#[derive(Debug)]
pub enum ControlNode {
    Empty(Empty),
    Enable(Enable),
    Seq(Seq),
    Par(Par),
    If(If),
    While(While),
    Invoke(Invoke),
}
