use std::{
    num::NonZeroU32,
    ops::{Add, Sub},
};

use super::{cell_prototype::CellPrototype, prelude::Identifier};
use crate::{
    flatten::structures::{
        environment::clock::ClockPair,
        index_trait::{impl_index, impl_index_nonzero, IndexRange, IndexRef},
        thread::ThreadIdx,
    },
    serialization::PrintCode,
};
use baa::{BitVecOps, BitVecValue};
use std::collections::HashSet;

// making these all u32 for now, can give the macro an optional type as the
// second arg to contract or expand as needed

/// The identifier for a component definition
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
pub struct ComponentIdx(u32);
impl_index!(ComponentIdx);

/// An index for auxiliary definition information for cells. This is used to
/// index into the [`SecondaryContext`][]
///
/// [`SecondaryContext`]: crate::flatten::structures::context::SecondaryContext::local_cell_defs
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct CellDefinitionIdx(u32);
impl_index!(CellDefinitionIdx);

/// An index for auxiliary definition information for ports. This is used to
/// index into the [`SecondaryContext`][]
///
/// [`SecondaryContext`]: crate::flatten::structures::context::SecondaryContext::local_port_defs
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct PortDefinitionIdx(u32);
impl_index!(PortDefinitionIdx);

/// An index for auxiliary definition information for ref cells. This is used to
/// index into the [`SecondaryContext`][]
///
/// [`SecondaryContext`]: crate::flatten::structures::context::SecondaryContext::ref_cell_defs
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct RefCellDefinitionIdx(u32);
impl_index!(RefCellDefinitionIdx);

/// An index for auxiliary definition information for ref ports. This is used to
/// index into the [`SecondaryContext`][]
///
/// [`SecondaryContext`]: crate::flatten::structures::context::SecondaryContext::ref_port_defs
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct RefPortDefinitionIdx(u32);
impl_index!(RefPortDefinitionIdx);

// Global indices

/// The index of a port instance in the global value map. Used to index into the [`Environment`][]
///
/// [`Environment`]: crate::flatten::structures::environment::Environment
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalPortIdx(NonZeroU32);
impl_index_nonzero!(GlobalPortIdx);

/// The index of a cell instance in the global value map. Used to index into the [`Environment`][]
///
/// [`Environment`]: crate::flatten::structures::environment::Environment
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalCellIdx(NonZeroU32);
impl_index_nonzero!(GlobalCellIdx);

/// The index of a ref cell instance in the global value map. Used to index into the [`Environment`][]
///
/// [`Environment`]: crate::flatten::structures::environment::Environment
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalRefCellIdx(u32);
impl_index!(GlobalRefCellIdx);

/// The index of a ref port instance in the global value map. Used to index into the [`Environment`][]
///
/// [`Environment`]: crate::flatten::structures::environment::Environment
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlobalRefPortIdx(u32);
impl_index!(GlobalRefPortIdx);

// Offset indices

/// A local port offset for a component.
///
/// These are used in the definition of assignments and can only be understood
/// in the context of the component they are defined under. Combined with a base
/// index from a component instance this can be resolved to a [`GlobalPortIdx`].
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalPortOffset(u32);
impl_index!(LocalPortOffset);

/// A local ref port offset for a component.
///
/// These are used in the definition of assignments and can only be understood
/// in the context of the component they are defined under. Combined with a base
/// index from a component instance this can be resolved to a
/// [`GlobalRefPortIdx`].
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PortRef {
    /// A port belonging to a non-ref cell/group in the current component or the
    /// component itself
    Local(LocalPortOffset),
    /// A port belonging to a ref cell in the current component
    Ref(LocalRefPortOffset),
}

impl PortRef {
    /// Returns the local offset of the port reference if it is a local port
    /// reference. Otherwise returns `None`.
    #[must_use]
    pub fn as_local(&self) -> Option<&LocalPortOffset> {
        if let Self::Local(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the local offset of the port reference if it is a ref port
    /// reference. Otherwise returns `None`.
    #[must_use]
    pub fn as_ref(&self) -> Option<&LocalRefPortOffset> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the local port offset of the port reference if it is a local port
    /// reference. Otherwise panics.
    pub fn unwrap_local(&self) -> &LocalPortOffset {
        self.as_local().unwrap()
    }

    /// Returns the local ref port offset of the port reference if it is a ref port
    /// reference. Otherwise panics.
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
    /// Constructs a global port reference from a local port reference and a base
    /// index.
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
    /// Returns the global port index of the port reference if it is a global port
    /// reference. Otherwise returns `None`.
    #[must_use]
    pub fn as_port(&self) -> Option<&GlobalPortIdx> {
        if let Self::Port(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
/// An enum wrapping the two different types of port definitions (ref/local)
pub enum PortDefinitionRef {
    /// A local port definition
    Local(PortDefinitionIdx),
    /// A ref port definition
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
    /// A local cell offset
    Local(LocalCellOffset),
    /// A ref cell offset
    Ref(LocalRefCellOffset),
}

impl CellRef {
    /// Returns the local cell offset if it is a local cell reference. Otherwise
    /// returns `None`.
    #[must_use]
    pub fn as_local(&self) -> Option<&LocalCellOffset> {
        if let Self::Local(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the local ref cell offset if it is a ref cell reference. Otherwise
    /// returns `None`.
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

/// An enum wrapping the two different type of global cell references
/// (ref/local). This is the global analogue to [CellRef].
#[derive(Debug)]
pub enum GlobalCellRef {
    /// A global cell index
    Cell(GlobalCellIdx),
    /// A global ref cell index
    Ref(GlobalRefCellIdx),
}

impl From<GlobalRefCellIdx> for GlobalCellRef {
    fn from(v: GlobalRefCellIdx) -> Self {
        Self::Ref(v)
    }
}

impl From<GlobalCellIdx> for GlobalCellRef {
    fn from(v: GlobalCellIdx) -> Self {
        Self::Cell(v)
    }
}

impl GlobalCellRef {
    /// Constructs a global cell reference from a local cell reference and a base
    /// index.
    pub fn from_local(local: CellRef, base_info: &BaseIndices) -> Self {
        match local {
            CellRef::Local(l) => (base_info + l).into(),
            CellRef::Ref(r) => (base_info + r).into(),
        }
    }

    /// Returns the global cell index if the reference is a global cell
    /// reference. Otherwise returns `None`.
    #[must_use]
    pub fn as_cell(&self) -> Option<&GlobalCellIdx> {
        if let Self::Cell(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the global ref cell index if the reference is a global ref cell
    /// reference. Otherwise returns `None`.
    #[must_use]
    pub fn as_ref(&self) -> Option<&GlobalRefCellIdx> {
        if let Self::Ref(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the global cell ref is [`Cell`][].
    ///
    /// [`Cell`]: GlobalCellRef::Cell
    #[must_use]
    pub fn is_cell(&self) -> bool {
        matches!(self, Self::Cell(..))
    }

    /// Returns `true` if the global cell ref is [`Ref`][].
    ///
    /// [`Ref`]: GlobalCellRef::Ref
    #[must_use]
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(..))
    }
}

/// An enum wrapping the two different type of cell definitions (ref/local)
pub enum CellDefinitionRef {
    /// A local cell definition
    Local(CellDefinitionIdx),
    /// A ref cell definition
    Ref(RefCellDefinitionIdx),
}

impl From<RefCellDefinitionIdx> for CellDefinitionRef {
    fn from(v: RefCellDefinitionIdx) -> Self {
        Self::Ref(v)
    }
}

impl From<CellDefinitionIdx> for CellDefinitionRef {
    fn from(v: CellDefinitionIdx) -> Self {
        Self::Local(v)
    }
}

/// A global index for assignments in the IR
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct AssignmentIdx(u32);
impl_index!(AssignmentIdx);

/// An enum representing the "winner" of an assignment.
///
/// This tells us how the value was assigned to the port. For standard
/// assignments, this is also used to detect conflicts where there are multiple
/// driving assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentWinner {
    /// Indicates that the "winning" assignment for this port was produced by a
    /// cell computation rather than an assignment. Since cells cannot share
    /// ports, there is no way for multiple cells to write the same output port,
    /// thus we don't need to record the cell that assigned it.
    Cell,
    /// A concrete value produced by the control program or from an external
    /// source
    Implicit,
    /// The assignment that produced this value.
    Assign(AssignmentIdx, GlobalCellIdx),
}

impl AssignmentWinner {
    #[must_use]
    pub fn as_assign(&self) -> Option<&AssignmentIdx> {
        if let Self::Assign(v, _c) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<(AssignmentIdx, GlobalCellIdx)> for AssignmentWinner {
    fn from((v, c): (AssignmentIdx, GlobalCellIdx)) -> Self {
        Self::Assign(v, c)
    }
}

impl From<(GlobalCellIdx, AssignmentIdx)> for AssignmentWinner {
    fn from((c, v): (GlobalCellIdx, AssignmentIdx)) -> Self {
        Self::Assign(v, c)
    }
}

/// A struct representing a value that has been assigned to a port. It wraps a
/// concrete value and the "winner" which assigned it.
#[derive(Clone)]
pub struct AssignedValue {
    val: BitVecValue,
    winner: AssignmentWinner,
    thread: Option<ThreadIdx>,
    clocks: Option<ClockPair>,
    propagate_clocks: bool,
    transitive_clocks: Option<HashSet<ClockPair>>,
}

impl AssignedValue {
    pub fn eq_no_transitive_clocks(&self, other: &Self) -> bool {
        self.val == other.val
            && self.winner == other.winner
            && self.thread == other.thread
            && self.clocks == other.clocks
            && self.propagate_clocks == other.propagate_clocks
    }
}

impl std::fmt::Debug for AssignedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssignedValue")
            .field("val", &self.val.to_bit_str())
            .field("winner", &self.winner)
            .field("thread", &self.thread)
            .field("clocks", &self.clocks)
            .field("propagate_clocks", &self.propagate_clocks)
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
    /// Creates a new AssignedValue
    pub fn new<T: Into<AssignmentWinner>>(val: BitVecValue, winner: T) -> Self {
        Self {
            val,
            winner: winner.into(),
            thread: None,
            clocks: None,
            propagate_clocks: false,
            transitive_clocks: None,
        }
    }

    /// Adds a clock to the set of transitive reads associated with this value
    pub fn add_transitive_clock(&mut self, clock_pair: ClockPair) {
        self.transitive_clocks
            .get_or_insert_with(Default::default)
            .insert(clock_pair);
    }

    pub fn add_transitive_clocks<I: IntoIterator<Item = ClockPair>>(
        &mut self,
        clocks: I,
    ) {
        self.transitive_clocks
            .get_or_insert_with(Default::default)
            .extend(clocks);
    }

    pub fn iter_transitive_clocks(
        &self,
    ) -> impl Iterator<Item = ClockPair> + '_ {
        self.transitive_clocks
            .as_ref()
            .map(|set| set.iter().copied())
            .into_iter()
            .flatten()
    }

    pub fn with_thread(mut self, thread: ThreadIdx) -> Self {
        self.thread = Some(thread);
        self
    }

    pub fn with_thread_optional(mut self, thread: Option<ThreadIdx>) -> Self {
        self.thread = thread;
        self
    }

    pub fn with_clocks(mut self, clock_pair: ClockPair) -> Self {
        self.clocks = Some(clock_pair);
        self
    }

    pub fn with_clocks_optional(
        mut self,
        clock_pair: Option<ClockPair>,
    ) -> Self {
        self.clocks = clock_pair;
        self
    }

    pub fn with_transitive_clocks_opt(
        mut self,
        clocks: Option<HashSet<ClockPair>>,
    ) -> Self {
        self.transitive_clocks = clocks;
        self
    }

    pub fn with_propagate_clocks(mut self) -> Self {
        self.propagate_clocks = true;
        self
    }

    pub fn set_propagate_clocks(&mut self, propagate_clocks: bool) {
        self.propagate_clocks = propagate_clocks;
    }

    /// Returns true if the two AssignedValues do not have the same winner
    pub fn has_conflict_with(&self, other: &Self) -> bool {
        self.winner != other.winner
    }

    /// Returns the value of the assigned value
    pub fn val(&self) -> &BitVecValue {
        &self.val
    }

    /// Returns the winner of the assigned value
    pub fn winner(&self) -> &AssignmentWinner {
        &self.winner
    }

    /// A utility constructor which returns a new implicitly assigned value with
    /// a one bit high value
    pub fn implicit_bit_high() -> Self {
        Self::new(BitVecValue::tru(), AssignmentWinner::Implicit)
    }

    /// A utility constructor which returns an [`AssignedValue`] with the given
    /// value and a [`AssignmentWinner::Cell`] as the winner
    #[inline]
    pub fn cell_value(val: BitVecValue) -> Self {
        Self::new(val, AssignmentWinner::Cell)
    }

    /// A utility constructor which returns an [`AssignedValue`] with the given
    /// value and a [`AssignmentWinner::Implicit`] as the winner
    #[inline]
    pub fn implicit_value(val: BitVecValue) -> Self {
        Self::new(val, AssignmentWinner::Implicit)
    }

    /// A utility constructor which returns an [`AssignedValue`] with a one bit
    /// high value and a [`AssignmentWinner::Cell`] as the winner
    #[inline]
    pub fn cell_b_high() -> Self {
        Self::cell_value(BitVecValue::tru())
    }
    /// A utility constructor which returns an [`AssignedValue`] with a one bit
    /// low value and a [`AssignmentWinner::Cell`] as the winner
    #[inline]
    pub fn cell_b_low() -> Self {
        Self::cell_value(BitVecValue::fals())
    }

    pub fn thread(&self) -> Option<ThreadIdx> {
        self.thread
    }

    pub fn clocks(&self) -> Option<&ClockPair> {
        self.clocks.as_ref()
    }

    pub fn transitive_clocks(&self) -> Option<&HashSet<ClockPair>> {
        self.transitive_clocks.as_ref()
    }

    pub fn propagate_clocks(&self) -> bool {
        self.propagate_clocks
    }
}

#[derive(Debug, Clone, Default)]
/// A wrapper struct around an option of an [AssignedValue]. In the case where
/// the option is [`None`], the value is taken to be undefined.
pub struct PortValue(Option<AssignedValue>);

impl std::fmt::Display for PortValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_value(PrintCode::Unsigned))
    }
}

impl PortValue {
    /// Returns true if the value is undefined. This is the inverse of
    /// [`PortValue::is_def`]
    pub fn is_undef(&self) -> bool {
        self.0.is_none()
    }

    /// Returns true if the value is defined. This is the inverse of
    /// [`PortValue::is_undef`]
    pub fn is_def(&self) -> bool {
        self.0.is_some()
    }

    /// Returns a reference to the underlying [`AssignedValue`] if it is defined.
    /// Otherwise returns `None`.
    pub fn as_option(&self) -> Option<&AssignedValue> {
        self.0.as_ref()
    }

    /// Returns a mutable reference to the underlying [`AssignedValue`] if it is
    /// defined. Otherwise returns `None`.
    pub fn as_option_mut(&mut self) -> Option<&mut AssignedValue> {
        self.0.as_mut()
    }

    /// Returns the underlying [`AssignedValue`] if it is defined. Otherwise
    /// returns `None`.
    pub fn into_option(self) -> Option<AssignedValue> {
        self.0
    }

    pub fn with_thread(mut self, thread: ThreadIdx) -> Self {
        if let Some(val) = self.0.as_mut() {
            val.thread = Some(thread);
        }
        self
    }

    pub fn with_thread_optional(mut self, thread: Option<ThreadIdx>) -> Self {
        if let Some(val) = self.0.as_mut() {
            val.thread = thread;
        }
        self
    }

    pub fn transitive_clocks(&self) -> Option<&HashSet<ClockPair>> {
        self.0.as_ref().and_then(|x| x.transitive_clocks())
    }

    /// If the value is defined, returns the value cast to a boolean. Otherwise
    /// returns `None`. It will panic if the given value is not one bit wide.
    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_ref().map(|x| x.val().to_bool().unwrap())
    }

    /// If the value is defined, returns the value cast to a u64. Otherwise,
    /// returns `None`. It uses the [`BitVecValue::to_u64`] method.
    pub fn as_u64(&self) -> Option<u64> {
        self.0.as_ref().map(|x| x.val().to_u64().unwrap())
    }

    pub fn is_zero(&self) -> Option<bool> {
        self.0.as_ref().map(|x| x.val.is_zero())
    }

    /// Returns a reference to the underlying value if it is defined. Otherwise
    /// returns `None`.
    pub fn val(&self) -> Option<&BitVecValue> {
        self.0.as_ref().map(|x| &x.val)
    }

    pub fn clocks(&self) -> Option<ClockPair> {
        self.0.as_ref().and_then(|x| x.clocks)
    }

    /// Returns a reference to the underlying [`AssignmentWinner`] if it is
    /// defined. Otherwise returns `None`.
    pub fn winner(&self) -> Option<&AssignmentWinner> {
        self.0.as_ref().map(|x| &x.winner)
    }

    /// Creates a new PortValue from the given value
    pub fn new<T: Into<Self>>(val: T) -> Self {
        val.into()
    }

    /// Creates a new undefined [PortValue]
    pub fn new_undef() -> Self {
        Self(None)
    }

    /// Creates a [PortValue] that has the "winner" as a cell
    pub fn new_cell(val: BitVecValue) -> Self {
        Self(Some(AssignedValue::cell_value(val)))
    }

    /// Creates a width-bit zero [PortValue] that has the "winner" as a cell
    pub fn new_cell_zeroes(width: u32) -> Self {
        Self::new_cell(BitVecValue::zero(width))
    }

    /// Creates a [PortValue] that has the "winner" as implicit
    pub fn new_implicit(val: BitVecValue) -> Self {
        Self(Some(AssignedValue::implicit_value(val)))
    }

    /// Sets the value to undefined and returns the former value if present.
    /// This is equivalent to [Option::take]
    pub fn set_undef(&mut self) -> Option<AssignedValue> {
        self.0.take()
    }

    /// Formats the value according to the given [PrintCode] and returns the
    /// resultant string. This is used by the debugger.
    pub fn format_value(&self, print_code: PrintCode) -> String {
        if let Some(v) = self.0.as_ref() {
            let v = &v.val;
            match print_code {
                PrintCode::Unsigned => format!("{}", v.to_big_uint()),
                PrintCode::Signed => format!("{}", v.to_big_int()),
                PrintCode::UFixed(num) => {
                    format!("{}", v.to_unsigned_fixed_point(num).unwrap())
                }
                PrintCode::SFixed(num) => {
                    format!("{}", v.to_signed_fixed_point(num).unwrap())
                }
                PrintCode::Binary => v.to_bit_str(),
            }
        } else {
            "undef".to_string()
        }
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

/// A struct containing information about a cell definition.
#[derive(Debug, Clone)]
pub struct CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    /// The name of the cell
    pub name: Identifier,
    /// The ports defined by the cell
    pub ports: IndexRange<C>,
    /// The component in which the cell is defined
    pub parent: ComponentIdx,
    /// The prototype of the cell
    pub prototype: CellPrototype,
    /// Whether the cell is marked with `@data`
    pub is_data: bool,
}

impl<C> CellDefinitionInfo<C>
where
    C: sealed::PortType,
{
    /// Constructs a new CellDefinitionInfo instance
    pub fn new(
        name: Identifier,
        ports: IndexRange<C>,
        parent: ComponentIdx,
        prototype: CellPrototype,
        is_data: bool,
    ) -> Self {
        Self {
            name,
            ports,
            parent,
            prototype,
            is_data,
        }
    }
}

/// A type alias for a local cell definition
pub type CellInfo = CellDefinitionInfo<LocalPortOffset>;
/// A type alias for a ref cell definition
pub type RefCellInfo = CellDefinitionInfo<LocalRefPortOffset>;

/// An enum wrapping the possible parents of a port
pub enum ParentIdx {
    /// The port is part of a component signature
    Component(ComponentIdx),
    /// The port belongs to a cell
    Cell(CellDefinitionIdx),
    /// The port belongs to a ref cell
    RefCell(RefCellDefinitionIdx),
    /// The port belongs to a group, i.e. is a hole
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

/// don't look at this. Seriously
mod sealed {
    use crate::flatten::structures::index_trait::IndexRef;

    use super::{LocalPortOffset, LocalRefPortOffset};

    pub trait PortType: IndexRef + PartialOrd {}

    impl PortType for LocalPortOffset {}
    impl PortType for LocalRefPortOffset {}
}

/// A struct wrapping the base index of all global port and cell indices. This
/// defines the start point for a component instance.
#[derive(Debug, Clone)]
pub struct BaseIndices {
    /// The local port starting index
    pub port_base: GlobalPortIdx,
    /// The local cell starting index
    pub cell_base: GlobalCellIdx,
    /// The ref cell starting index
    pub ref_cell_base: GlobalRefCellIdx,
    /// The ref port starting index
    pub ref_port_base: GlobalRefPortIdx,
}

impl BaseIndices {
    /// Creates a new BaseIndices instance
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
