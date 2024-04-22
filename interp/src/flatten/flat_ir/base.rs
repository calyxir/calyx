use std::{
    num::NonZeroU32,
    ops::{Add, Sub},
};

use crate::{
    flatten::structures::index_trait::{
        impl_index, impl_index_nonzero, IndexRange, IndexRef,
    },
    values::Value,
};

use super::{cell_prototype::CellPrototype, prelude::Identifier};

// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

/// The identifier for a component definition
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
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
pub struct GlobalPortIdx(NonZeroU32);
impl_index_nonzero!(GlobalPortIdx);

/// The index of a cell instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalCellIdx(NonZeroU32);
impl_index_nonzero!(GlobalCellIdx);

/// The index of a ref cell instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalRefCellIdx(u32);
impl_index!(GlobalRefCellIdx);

/// The index of a ref port instance in the global value map
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalRefPortIdx(u32);
impl_index!(GlobalRefPortIdx);

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

/// Enum used in assignments to encapsulate the different types of port
/// references these are always relative to a component's base-point and must be
/// converted to global references when used.
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

/// This is the global analogue to [PortRef] and contains global identifiers
/// after the relative offsets have been transformed via a component base location
pub enum GlobalPortRef {
    /// A non-ref port with an exact address
    Port(GlobalPortIdx),
    /// A reference port
    Ref(GlobalRefPortIdx),
}

impl GlobalPortRef {
    pub fn from_local(local: PortRef, base_info: &BaseIndices) -> Self {
        match local {
            PortRef::Local(l) => (base_info + l).into(),
            PortRef::Ref(r) => (base_info + r).into(),
        }
    }
}

impl From<GlobalRefPortIdx> for GlobalPortRef {
    fn from(v: GlobalRefPortIdx) -> Self {
        Self::Ref(v)
    }
}

impl From<GlobalPortIdx> for GlobalPortRef {
    fn from(v: GlobalPortIdx) -> Self {
        Self::Port(v)
    }
}

impl GlobalPortRef {
    #[must_use]
    pub fn _as_port(&self) -> Option<&GlobalPortIdx> {
        if let Self::Port(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn _as_ref(&self) -> Option<&GlobalRefPortIdx> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
/// An enum wrapping the two different types of port definitions (ref/local)
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

/// A wrapper enum distinguishing between local offsets to cells and ref cells
///
/// TODO griffin: do some clever bit stuff to pack this into a single u32 rather
/// than the 64 bits it current occupies due to the discriminant being 32 bits
/// because of alignment
#[derive(Debug, Copy, Clone)]
pub enum CellRef {
    Local(LocalCellOffset),
    Ref(LocalRefCellOffset),
}

impl CellRef {
    #[must_use]
    pub fn as_local(&self) -> Option<&LocalCellOffset> {
        if let Self::Local(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_ref(&self) -> Option<&LocalRefCellOffset> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }
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

/// A global index for assignments in the IR
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct AssignmentIdx(u32);
impl_index!(AssignmentIdx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentWinner {
    /// Indicates that the "winning" assignment for this port was produced by a
    /// cell computation rather than an assignment. Since cells cannot share
    /// ports, there is no way for multiple cells to write the same output port,
    /// thus we don't need to record the cell that assigned it.
    Cell,
    /// A concrete value produced by the control program
    Implicit,
    /// The assignment that produced this value.
    Assign(AssignmentIdx),
}

impl From<AssignmentIdx> for AssignmentWinner {
    fn from(v: AssignmentIdx) -> Self {
        Self::Assign(v)
    }
}

#[derive(Clone, PartialEq)]
pub struct AssignedValue {
    val: Value,
    winner: AssignmentWinner,
}

impl std::fmt::Debug for AssignedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssignedValue")
            .field("val", &format!("{}", &self.val))
            .field("winner", &self.winner)
            .finish()
    }
}

impl std::fmt::Display for AssignedValue {
    // TODO: replace with something more reasonable
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl AssignedValue {
    pub fn new<T: Into<AssignmentWinner>>(val: Value, winner: T) -> Self {
        Self {
            val,
            winner: winner.into(),
        }
    }

    /// Returns true if the two AssignedValues do not have the same winner
    pub fn has_conflict_with(&self, other: &Self) -> bool {
        self.winner != other.winner
    }

    pub fn val(&self) -> &Value {
        &self.val
    }

    pub fn winner(&self) -> &AssignmentWinner {
        &self.winner
    }

    pub fn implicit_bit_high() -> Self {
        Self {
            val: Value::bit_high(),
            winner: AssignmentWinner::Implicit,
        }
    }

    #[inline]
    pub fn cell_value(val: Value) -> Self {
        Self {
            val,
            winner: AssignmentWinner::Cell,
        }
    }

    #[inline]
    pub fn implicit_value(val: Value) -> Self {
        Self {
            val,
            winner: AssignmentWinner::Implicit,
        }
    }

    #[inline]
    pub fn cell_b_high() -> Self {
        Self::cell_value(Value::bit_high())
    }

    #[inline]
    pub fn cell_b_low() -> Self {
        Self::cell_value(Value::bit_low())
    }
}

#[derive(Debug, Clone, Default)]
/// A wrapper struct around an option of an [AssignedValue]
pub struct PortValue(Option<AssignedValue>);

impl std::fmt::Display for PortValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl PortValue {
    pub fn is_undef(&self) -> bool {
        self.0.is_none()
    }

    pub fn is_def(&self) -> bool {
        self.0.is_some()
    }

    pub fn as_option(&self) -> Option<&AssignedValue> {
        self.0.as_ref()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_ref().map(|x| x.val().as_bool())
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.0.as_ref().map(|x| x.val().as_usize())
    }

    pub fn val(&self) -> Option<&Value> {
        self.0.as_ref().map(|x| &x.val)
    }

    pub fn winner(&self) -> Option<&AssignmentWinner> {
        self.0.as_ref().map(|x| &x.winner)
    }

    pub fn new<T: Into<Self>>(val: T) -> Self {
        val.into()
    }

    pub fn new_undef() -> Self {
        Self(None)
    }

    /// Creates a [PortValue] that has the "winner" as a cell
    pub fn new_cell(val: Value) -> Self {
        Self(Some(AssignedValue::cell_value(val)))
    }

    /// Creates a [PortValue] that has the "winner" as implicit
    pub fn new_implicit(val: Value) -> Self {
        Self(Some(AssignedValue::implicit_value(val)))
    }

    /// Sets the value to undefined and returns the former value if present.
    /// This is equivalent to [Option::take]
    pub fn set_undef(&mut self) -> Option<AssignedValue> {
        self.0.take()
    }
}

impl From<Option<AssignedValue>> for PortValue {
    fn from(value: Option<AssignedValue>) -> Self {
        Self(value)
    }
}

impl From<AssignedValue> for PortValue {
    fn from(value: AssignedValue) -> Self {
        Self(Some(value))
    }
}

impl From<PortValue> for Option<AssignedValue> {
    fn from(value: PortValue) -> Self {
        value.0
    }
}

/// A global index for standard groups in the IR
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GroupIdx(u32);
impl_index!(GroupIdx);

/// A global index for combinational groups in the IR
///
/// This is non-zero to make the option-types of this index used in the IR If and
/// While nodes the same size as the index itself.
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct CombGroupIdx(NonZeroU32);
impl_index_nonzero!(CombGroupIdx);

/// A global index for guards used in the IR
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GuardIdx(u32);
impl_index!(GuardIdx);

#[derive(Debug, Clone)]
pub struct CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    pub name: Identifier,
    pub ports: IndexRange<C>,
    pub parent: ComponentIdx,
    pub prototype: CellPrototype,
}

impl<C> CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    pub fn new(
        name: Identifier,
        ports: IndexRange<C>,
        parent: ComponentIdx,
        prototype: CellPrototype,
    ) -> Self {
        Self {
            name,
            ports,
            parent,
            prototype,
        }
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

#[derive(Debug, Clone)]
pub struct BaseIndices {
    pub port_base: GlobalPortIdx,
    pub cell_base: GlobalCellIdx,
    pub ref_cell_base: GlobalRefCellIdx,
    pub ref_port_base: GlobalRefPortIdx,
}

impl BaseIndices {
    pub fn new(
        port_base: GlobalPortIdx,
        cell_base: GlobalCellIdx,
        ref_cell_base: GlobalRefCellIdx,
        ref_port_base: GlobalRefPortIdx,
    ) -> Self {
        Self {
            port_base,
            cell_base,
            ref_cell_base,
            ref_port_base,
        }
    }
}

impl Add<LocalPortOffset> for &BaseIndices {
    type Output = GlobalPortIdx;

    fn add(self, rhs: LocalPortOffset) -> Self::Output {
        GlobalPortIdx::new(self.port_base.index() + rhs.index())
    }
}

impl Add<LocalRefPortOffset> for &BaseIndices {
    type Output = GlobalRefPortIdx;

    fn add(self, rhs: LocalRefPortOffset) -> Self::Output {
        GlobalRefPortIdx::new(self.ref_port_base.index() + rhs.index())
    }
}

impl Add<LocalCellOffset> for &BaseIndices {
    type Output = GlobalCellIdx;

    fn add(self, rhs: LocalCellOffset) -> Self::Output {
        GlobalCellIdx::new(self.cell_base.index() + rhs.index())
    }
}

impl Add<LocalRefCellOffset> for &BaseIndices {
    type Output = GlobalRefCellIdx;

    fn add(self, rhs: LocalRefCellOffset) -> Self::Output {
        GlobalRefCellIdx::new(self.ref_cell_base.index() + rhs.index())
    }
}

impl Add<&LocalPortOffset> for &BaseIndices {
    type Output = GlobalPortIdx;

    fn add(self, rhs: &LocalPortOffset) -> Self::Output {
        GlobalPortIdx::new(self.port_base.index() + rhs.index())
    }
}

impl Add<&LocalRefPortOffset> for &BaseIndices {
    type Output = GlobalRefPortIdx;

    fn add(self, rhs: &LocalRefPortOffset) -> Self::Output {
        GlobalRefPortIdx::new(self.ref_port_base.index() + rhs.index())
    }
}

impl Add<&LocalCellOffset> for &BaseIndices {
    type Output = GlobalCellIdx;

    fn add(self, rhs: &LocalCellOffset) -> Self::Output {
        GlobalCellIdx::new(self.cell_base.index() + rhs.index())
    }
}

impl Add<&LocalRefCellOffset> for &BaseIndices {
    type Output = GlobalRefCellIdx;

    fn add(self, rhs: &LocalRefCellOffset) -> Self::Output {
        GlobalRefCellIdx::new(self.ref_cell_base.index() + rhs.index())
    }
}

impl Sub<&BaseIndices> for GlobalPortIdx {
    type Output = LocalPortOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalPortOffset::new(self.index() - rhs.port_base.index())
    }
}

impl Sub<&BaseIndices> for GlobalRefPortIdx {
    type Output = LocalRefPortOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalRefPortOffset::new(self.index() - rhs.ref_port_base.index())
    }
}

impl Sub<&BaseIndices> for GlobalCellIdx {
    type Output = LocalCellOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalCellOffset::new(self.index() - rhs.cell_base.index())
    }
}

impl Sub<&BaseIndices> for GlobalRefCellIdx {
    type Output = LocalRefCellOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalRefCellOffset::new(self.index() - rhs.ref_cell_base.index())
    }
}

impl Sub<&BaseIndices> for &GlobalPortIdx {
    type Output = LocalPortOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalPortOffset::new(self.index() - rhs.port_base.index())
    }
}

impl Sub<&BaseIndices> for &GlobalRefPortIdx {
    type Output = LocalRefPortOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalRefPortOffset::new(self.index() - rhs.ref_port_base.index())
    }
}

impl Sub<&BaseIndices> for &GlobalCellIdx {
    type Output = LocalCellOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalCellOffset::new(self.index() - rhs.cell_base.index())
    }
}

impl Sub<&BaseIndices> for &GlobalRefCellIdx {
    type Output = LocalRefCellOffset;

    fn sub(self, rhs: &BaseIndices) -> Self::Output {
        LocalRefCellOffset::new(self.index() - rhs.ref_cell_base.index())
    }
}
