use calyx_frontend::source_info::PositionId;
use cider_idx::{impl_index, iter::IndexRange, maps::IndexedMap};
use smallvec::SmallVec;

use crate::flatten::flat_ir::prelude::*;

/// An index representing a control statement
#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct ControlIdx(u32);
impl_index!(ControlIdx);

/// A map storing [ControlNodes](ControlNode) indexed by [ControlIdx]
pub type ControlMap = IndexedMap<ControlIdx, ControlNode>;

/// A vector of control indices
pub type CtrlVec = SmallVec<[ControlIdx; 4]>;

/// An empty control node
#[derive(Debug)]
pub struct Empty;

/// A group enable node. Analogue of [calyx_ir::Enable]
#[derive(Debug)]
pub struct Enable(GroupIdx);

impl Enable {
    /// Returns the group index of the enable statement
    pub fn group(&self) -> GroupIdx {
        self.0
    }

    /// Create a new enable statement
    pub fn new(group: GroupIdx) -> Self {
        Self(group)
    }
}

/// Sequence of control nodes. Analogue of [calyx_ir::Seq]
#[derive(Debug)]
pub struct Seq(CtrlVec);

impl Seq {
    /// Create a new sequence operator from an iterator of control indices
    pub fn new<S>(input: S) -> Self
    where
        S: Iterator<Item = ControlIdx>,
    {
        Self(input.collect())
    }

    /// Returns a reference to the control indices in the seq
    pub fn stms(&self) -> &[ControlIdx] {
        &self.0
    }

    /// Returns the number of control indices in the seq
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the seq is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Parallel compositions of control nodes. Analogue of [calyx_ir::Par]
#[derive(Debug)]
pub struct Par(CtrlVec);

impl Par {
    /// Create a new par from an iterator of control indices
    pub fn new<S>(input: S) -> Self
    where
        S: Iterator<Item = ControlIdx>,
    {
        Self(input.collect())
    }

    /// Returns a reference to the body of the par
    pub fn stms(&self) -> &[ControlIdx] {
        &self.0
    }

    /// Returns the number of arms/threads in this par
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if this par contains no arms/threads
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An if-then-else control node. Analogue of [calyx_ir::If]
#[derive(Debug)]
pub struct If {
    cond_port: PortRef,
    cond_group: Option<CombGroupIdx>,
    tbranch: ControlIdx,
    fbranch: ControlIdx,
}

impl If {
    /// Construct a new if statement
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

    /// Returns the port reference of the condition
    pub fn cond_port(&self) -> PortRef {
        self.cond_port
    }

    /// Returns the index of the `with` group if it exists
    pub fn cond_group(&self) -> Option<CombGroupIdx> {
        self.cond_group
    }

    /// Returns the index of the true branch
    pub fn tbranch(&self) -> ControlIdx {
        self.tbranch
    }

    /// Returns the index of the false branch
    pub fn fbranch(&self) -> ControlIdx {
        self.fbranch
    }
}

/// A while loop control node. Analogue of [calyx_ir::While]
#[derive(Debug)]
pub struct While {
    cond_port: PortRef,
    cond_group: Option<CombGroupIdx>,
    body: ControlIdx,
}

impl While {
    /// Construct a new while node
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

    /// Getter for the condition port
    pub fn cond_port(&self) -> PortRef {
        self.cond_port
    }

    /// Getter for the condition group, if present
    pub fn cond_group(&self) -> Option<CombGroupIdx> {
        self.cond_group
    }

    /// Getter for the loop body
    pub fn body(&self) -> ControlIdx {
        self.body
    }
}

/// A container for the signature of an invocation command.
#[derive(Debug)]
pub struct InvokeSignature {
    /// The ports attached to the input of the invoked cell, an association list
    /// of the port ref in the **PARENT** context, and the port connected
    /// to it in the parent context i.e. (dst, src)
    pub inputs: SmallVec<[(PortRef, PortRef); 1]>,
    /// The ports attached to the outputs of the invoked cell, an association list
    /// of the port ref in the **PARENT** context, and the port connected
    /// to it in the parent context. i.e. (src, dst)
    pub outputs: SmallVec<[(PortRef, PortRef); 1]>,
}

impl InvokeSignature {
    // TODO Griffin: fix this it's stupid
    /// Returns an iterator over the ports in the signature. Ports are given as
    /// (dest, src) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&PortRef, &PortRef)> {
        self.inputs
            .iter()
            .map(|x| (&x.0, &x.1))
            // need to reverse the outputs because invoke is confusing
            .chain(self.outputs.iter().map(|(src, dest)| (dest, src)))
    }
}

/// Invoke control node. Analogue of [calyx_ir::Invoke]
///
/// TODO Griffin: Consider making this smaller? Move ref_cells into signature box?
#[derive(Debug)]
pub struct Invoke {
    /// The cell being invoked
    pub cell: CellRef,
    /// Optional group enabled during invocation of the cell (the calyx `with`
    /// statement)
    pub comb_group: Option<CombGroupIdx>,
    /// The external cells passed as arguments to the invoked cell, an
    /// association list of the refcell offset in the invoked context, and the
    /// cell realizing it in the parent context
    pub ref_cells: SmallVec<[(LocalRefCellOffset, CellRef); 1]>,
    /// The signature (behind a box for space reasons). This is used during the
    /// flattening process and printing, but is not used during simulation as
    /// the assignments are pre-constructed from it.
    pub signature: Box<InvokeSignature>,
    /// The go port
    pub go: PortRef,
    /// The done port
    pub done: PortRef,
    /// The assignments implied by the invocation command. These are
    /// preconstructed during the flattening process.
    pub assignments: IndexRange<AssignmentIdx>,
}

impl Invoke {
    /// Create a new invoke node
    pub fn new<R, I, O>(
        cell: CellRef,
        comb_group: Option<CombGroupIdx>,
        ref_cells: R,
        inputs: I,
        outputs: O,
        go: PortRef,
        done: PortRef,
    ) -> Self
    where
        R: IntoIterator<Item = (LocalRefCellOffset, CellRef)>,
        I: IntoIterator<Item = (PortRef, PortRef)>,
        O: IntoIterator<Item = (PortRef, PortRef)>,
    {
        Self {
            cell,
            comb_group,
            ref_cells: ref_cells.into_iter().collect(),
            signature: Box::new(InvokeSignature {
                inputs: inputs.into_iter().collect(),
                outputs: outputs.into_iter().collect(),
            }),
            go,
            done,
            assignments: IndexRange::empty_interval(),
        }
    }
}

/// A bounded loop
#[derive(Debug)]
pub struct Repeat {
    /// The loop body
    pub body: ControlIdx,
    /// The number of times to repeat the loop body
    pub num_repeats: u64,
}

impl Repeat {
    /// Create a new bounded loop control node
    pub fn new(body: ControlIdx, num_repeats: u64) -> Self {
        Self { body, num_repeats }
    }
}

/// An enum representing the different types of control nodes. Analogue of [calyx_ir::Control]
#[derive(Debug)]
pub enum Control {
    /// An empty control node
    Empty(Empty),
    /// A group enable node
    Enable(Enable),
    /// A sequential composition
    Seq(Seq),
    /// A parallel composition
    Par(Par),
    /// An if-then-else control node
    If(If),
    /// A while loop control node
    While(While),
    /// A bounded loop control node
    Repeat(Repeat),
    /// An invoke control node
    Invoke(Invoke),
}

impl Control {
    /// Returns true if the control node is a leaf node (i.e. invoke, enable, or
    /// empty)
    pub fn is_leaf(&self) -> bool {
        match self {
            Control::While(_)
            | Control::Repeat(_)
            | Control::Seq(_)
            | Control::Par(_)
            | Control::If(_) => false,
            Control::Enable(_) | Control::Invoke(_) | Control::Empty(_) => true,
        }
    }
}

#[derive(Debug)]
pub struct ControlNode {
    pub control: Control,
    pub pos: Option<Box<[PositionId]>>,
}

impl AsRef<Control> for ControlNode {
    fn as_ref(&self) -> &Control {
        &self.control
    }
}

impl ControlNode {
    pub fn positions(&self) -> impl Iterator<Item = PositionId> {
        self.pos.iter().flatten().copied()
    }
}

impl std::ops::Deref for ControlNode {
    type Target = Control;

    fn deref(&self) -> &Self::Target {
        &self.control
    }
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
