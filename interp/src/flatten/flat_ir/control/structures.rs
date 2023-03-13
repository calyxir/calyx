use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::{index_trait::impl_index, indexed_map::IndexedMap},
};

impl_index!(pub ControlIdx);
pub type ControlMap = IndexedMap<ControlIdx, ControlNode>;

pub type CtrlVec = SmallVec<[ControlIdx; 4]>;

#[derive(Debug)]
pub struct Empty;

#[derive(Debug)]
pub struct Enable(GroupIdx);

impl Enable {
    pub fn group(&self) -> GroupIdx {
        self.0
    }

    pub fn new(group: GroupIdx) -> Self {
        Self(group)
    }
}

#[derive(Debug)]
pub struct Seq(CtrlVec);

impl Seq {
    pub fn new<S>(input: S) -> Self
    where
        S: Iterator<Item = ControlIdx>,
    {
        Self(input.collect())
    }

    pub fn stms(&self) -> &[ControlIdx] {
        &self.0
    }
}

#[derive(Debug)]
pub struct Par(CtrlVec);

impl Par {
    pub fn new<S>(input: S) -> Self
    where
        S: Iterator<Item = ControlIdx>,
    {
        Self(input.collect())
    }

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
    pub fn new(
        cond_port: PortRef,
        cond_group: Option<CombGroupIdx>,
        tbranch: ControlIdx,
        fbranch: ControlIdx,
    ) -> Self {
        Self {
            cond_port,
            cond_group,
            tbranch,
            fbranch,
        }
    }

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
    pub fn new(
        cond_port: PortRef,
        cond_group: Option<CombGroupIdx>,
        body: ControlIdx,
    ) -> Self {
        Self {
            cond_port,
            cond_group,
            body,
        }
    }

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

// ---------------------

/// An enum indicating whether an entity is entirely local to the given context
/// or a reference from another context (i.e. refcell or port on a refcell)
pub(crate) enum ContainmentType {
    /// A local cell/port
    Local,
    /// A ref cell/port
    Ref,
}

impl ContainmentType {
    /// Returns `true` if the containment type is [`Local`].
    ///
    /// [`Local`]: ContainmentType::Local
    #[must_use]
    pub(crate) fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    /// Returns `true` if the containment type is [`Ref`].
    ///
    /// [`Ref`]: ContainmentType::Ref
    #[must_use]
    pub(crate) fn is_ref(&self) -> bool {
        matches!(self, Self::Ref)
    }
}
