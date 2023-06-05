use crate::flatten::structures::index_trait::{
    impl_index, impl_index_nonzero, IndexRange,
};

use super::prelude::Identifier;

// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

impl_index!(pub ComponentRef);

// Definition indexes, used to address information
impl_index!(pub CellDefinitionIdx);
impl_index!(pub PortDefinitionIdx);
impl_index!(pub RefCellDefinitionIdx);
impl_index!(pub RefPortDefinitionIdx);

// Global indices
impl_index!(pub GlobalPortId);
impl_index!(pub GlobalCellId);
impl_index!(pub GlobalRefCellId);

// Offset indices
impl_index!(pub LocalPortOffset);
impl_index!(pub LocalRefPortOffset);
impl_index!(pub LocalCellOffset);
impl_index!(pub LocalRefCellOffset);

#[derive(Debug, Copy, Clone)]
pub enum PortRef {
    Local(LocalPortOffset),
    Ref(LocalRefPortOffset),
}

impl PortRef {
    #[must_use]
    pub fn as_local(&self) -> Option<&LocalPortOffset> {
        if let Self::Local(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_ref(&self) -> Option<&LocalRefPortOffset> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn unwrap_local(&self) -> &LocalPortOffset {
        self.as_local().unwrap()
    }

    pub fn unwrap_ref(&self) -> &LocalRefPortOffset {
        self.as_ref().unwrap()
    }
}

impl From<LocalRefPortOffset> for PortRef {
    fn from(v: LocalRefPortOffset) -> Self {
        Self::Ref(v)
    }
}

impl From<LocalPortOffset> for PortRef {
    fn from(v: LocalPortOffset) -> Self {
        Self::Local(v)
    }
}

pub enum PortDefinitionRef {
    Local(PortDefinitionIdx),
    Ref(RefPortDefinitionIdx),
}

impl From<RefPortDefinitionIdx> for PortDefinitionRef {
    fn from(v: RefPortDefinitionIdx) -> Self {
        Self::Ref(v)
    }
}

impl From<PortDefinitionIdx> for PortDefinitionRef {
    fn from(v: PortDefinitionIdx) -> Self {
        Self::Local(v)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CellRef {
    Local(LocalCellOffset),
    Ref(LocalRefCellOffset),
}

impl From<LocalRefCellOffset> for CellRef {
    fn from(v: LocalRefCellOffset) -> Self {
        Self::Ref(v)
    }
}

impl From<LocalCellOffset> for CellRef {
    fn from(v: LocalCellOffset) -> Self {
        Self::Local(v)
    }
}

impl_index!(pub AssignmentIdx);

impl_index!(pub GroupIdx);

// This is non-zero to make the option-types of this index used in the IR If and
// While nodes the same size as the index itself.
impl_index_nonzero!(pub CombGroupIdx);

impl_index!(pub GuardIdx);

#[derive(Debug, Clone)]
pub struct CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    name: Identifier,
    ports: IndexRange<C>,
    parent: ComponentRef,
}

impl<C> CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    pub fn new(
        name: Identifier,
        ports: IndexRange<C>,
        parent: ComponentRef,
    ) -> Self {
        Self {
            name,
            ports,
            parent,
        }
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn ports(&self) -> &IndexRange<C> {
        &self.ports
    }
}

pub type CellInfo = CellDefinitionInfo<LocalPortOffset>;
pub type RefCellInfo = CellDefinitionInfo<LocalRefPortOffset>;

pub enum ParentIdx {
    Component(ComponentRef),
    Cell(CellDefinitionIdx),
    RefCell(RefCellDefinitionIdx),
    Group(GroupIdx),
}

impl From<GroupIdx> for ParentIdx {
    fn from(v: GroupIdx) -> Self {
        Self::Group(v)
    }
}

impl From<RefCellDefinitionIdx> for ParentIdx {
    fn from(v: RefCellDefinitionIdx) -> Self {
        Self::RefCell(v)
    }
}

impl From<CellDefinitionIdx> for ParentIdx {
    fn from(v: CellDefinitionIdx) -> Self {
        Self::Cell(v)
    }
}

impl From<ComponentRef> for ParentIdx {
    fn from(v: ComponentRef) -> Self {
        Self::Component(v)
    }
}

// don't look at this. Seriously
mod sealed {
    use crate::flatten::structures::index_trait::IndexRef;

    use super::{LocalPortOffset, LocalRefPortOffset};

    pub trait PortType: IndexRef + PartialOrd {}

    impl PortType for LocalPortOffset {}
    impl PortType for LocalRefPortOffset {}
}
