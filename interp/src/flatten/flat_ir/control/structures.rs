use smallvec::SmallVec;

use crate::flatten::{
    flat_ir::prelude::*,
    structures::{index_trait::impl_index, indexed_map::IndexedMap},
};

#[derive(Debug, Eq, Copy, Clone, PartialEq)]
pub struct ControlIdx(u32);
impl_index!(ControlIdx);

/// A map storing [ControlNodes](ControlNode) indexed by [ControlIdx]
pub type ControlMap = IndexedMap<ControlIdx, ControlNode>;

/// A vector of control indices
pub type CtrlVec = SmallVec<[ControlIdx; 4]>;

/// An empty control node
#[derive(Debug)]
pub struct Empty;

/// A group enable node
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

/// Sequence of control nodes
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

/// Parallel compositions of control nodes
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

/// An if-then-else control node
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

/// A while loop control node
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

/// Invoke control node
#[derive(Debug)]
pub struct Invoke {
    /// The cell being invoked
    cell: LocalCellOffset,
    /// Optional group enabled during invocation of the cell (the calyx `with`
    /// statement)
    comb_group: Option<CombGroupIdx>,
    /// The external cells passed as arguments to the invoked cell, an
    /// association list of the refcell offset in the invoked context, and the
    /// cell realizing it in the parent context
    ref_cells: SmallVec<[(LocalRefCellOffset, CellRef); 1]>,
    /// The ports attached to the input of the invoked cell, an association list
    /// of the local port offset in the **PARENT** context, and the port connected
    /// to it in the parent context
    inputs: SmallVec<[(LocalPortOffset, PortRef); 1]>,
    /// The ports attached to the outputs of the invoked cell, an association list
    /// of the local port offset in the **PARENT** context, and the port connected
    /// to it in the parent context
    outputs: SmallVec<[(LocalPortOffset, PortRef); 1]>,
}

impl Invoke {
    pub fn new<R, I, O>(
        cell: LocalCellOffset,
        comb_group: Option<CombGroupIdx>,
        ref_cells: R,
        inputs: I,
        outputs: O,
    ) -> Self
    where
        R: IntoIterator<Item = (LocalRefCellOffset, CellRef)>,
        I: IntoIterator<Item = (LocalPortOffset, PortRef)>,
        O: IntoIterator<Item = (LocalPortOffset, PortRef)>,
    {
        Self {
            cell,
            comb_group,
            ref_cells: ref_cells.into_iter().collect(),
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
        }
    }
}

/// An enum representing the different types of control nodes
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
