use std::num::NonZeroU32;

use crate::flatten::structures::index_trait::{
    impl_index, impl_index_nonzero, IndexRange,
};

use super::prelude::Identifier;

// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

/// The identifier for a component definition
#[derive(Debug, Eq, Copy, Clone, PartialEq)]
pub struct ComponentIdx(u32);
impl_index!(ComponentIdx);

/// An index for auxillary definition information for cells
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct CellDefinitionIdx(u32);
impl_index!(CellDefinitionIdx);

/// An index for auxillary definition information for ports
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct PortDefinitionIdx(u32);
impl_index!(PortDefinitionIdx);

/// An index for auxillary definition information for ref cells
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct RefCellDefinitionIdx(u32);
impl_index!(RefCellDefinitionIdx);

/// An index for auxillary definition information for ref ports
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct RefPortDefinitionIdx(u32);
impl_index!(RefPortDefinitionIdx);

// Global indices

/// The index of a port instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalPortId(u32);
impl_index!(GlobalPortId);

/// The index of a cell instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalCellId(u32);
impl_index!(GlobalCellId);

/// The index of a ref cell instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalRefCellId(u32);
impl_index!(GlobalRefCellId);

// Offset indices

/// A local port offset for a component. These are used in the definition of
/// assignments and can only be understood in the context of the component they
/// are defined under.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalPortOffset(u32);
impl_index!(LocalPortOffset);

/// A local ref port offset for a component. These are used in the definition of
/// assignments and can only be understood in the context of the component they
/// are defined under.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalRefPortOffset(u32);
impl_index!(LocalRefPortOffset);

/// A local cell offset for a component. Primarily for alignment bookkeeping.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalCellOffset(u32);
impl_index!(LocalCellOffset);

/// A local ref cell offset for a component. Primarily for alignment bookkeeping.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalRefCellOffset(u32);
impl_index!(LocalRefCellOffset);

/// Enum used in assignments to encapsulate the different types of port references
#[derive(Debug, Copy, Clone)]
pub enum PortRef {
    /// A port belonging to a non-ref cell/group in the current component or the
    /// component itself
    Local(LocalPortOffset),
    /// A port belonging to a ref cell in the current component
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

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct AssignmentIdx(u32);
impl_index!(AssignmentIdx);

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GroupIdx(u32);
impl_index!(GroupIdx);

// This is non-zero to make the option-types of this index used in the IR If and
// While nodes the same size as the index itself.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct CombGroupIdx(NonZeroU32);
impl_index_nonzero!(CombGroupIdx);

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GuardIdx(u32);
impl_index!(GuardIdx);

#[derive(Debug, Clone)]
pub struct CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    name: Identifier,
    ports: IndexRange<C>,
    parent: ComponentIdx,
}

impl<C> CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    pub fn new(
        name: Identifier,
        ports: IndexRange<C>,
        parent: ComponentIdx,
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
    Component(ComponentIdx),
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

impl From<ComponentIdx> for ParentIdx {
    fn from(v: ComponentIdx) -> Self {
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
