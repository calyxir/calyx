use std::collections::hash_map::Entry;

use ahash::{HashMap, HashMapExt};
use cider_idx::iter::IndexRange;
use smallvec::SmallVec;

use super::super::context::Context;
use crate::flatten::{
    flat_ir::prelude::{
        AssignmentIdx, CombGroupIdx, Control, ControlIdx, GlobalCellIdx,
    },
    structures::thread::ThreadIdx,
};

/// Simple struct containing both the component instance and the active leaf
/// node in the component. This is used to represent an active execution of some
/// portion of the control tree
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ControlPoint {
    pub comp: GlobalCellIdx,
    pub control_node_idx: ControlIdx,
}

impl ControlPoint {
    pub fn new(comp: GlobalCellIdx, control_leaf: ControlIdx) -> Self {
        Self {
            comp,
            control_node_idx: control_leaf,
        }
    }

    /// Constructs a new [ControlPoint] from an existing one by copying over the
    /// component identifier but changing the leaf node
    pub fn new_retain_comp(&self, target: ControlIdx) -> Self {
        Self {
            comp: self.comp,
            control_node_idx: target,
        }
    }

    pub fn get_next(node: &Self, ctx: &Context) -> Option<Self> {
        let mut current = ctx.primary[node.control_node_idx].parent?;
        let mut prior = node.control_node_idx;
        let mut out = None;

        while out.is_none() {
            match &ctx.primary[current].control {
                Control::Seq(seq) => {
                    let idx = seq.find_child(|&child| child == prior).unwrap();

                    if idx + 1 < seq.stms().len() {
                        out = Some(seq.stms()[idx + 1])
                    }
                }
                Control::If(i) => {
                    if i.cond_group().is_some() {
                        // since this has a with, we need to re-visit
                        // the node to clean-up the with group
                        out = Some(current);
                    }
                    // no cleanup needed, just keep searching up the tree
                }
                // Need to recheck loop condition
                Control::While(_) | Control::Repeat(_) => {
                    out = Some(current);
                }
                // Need to check if the par is done
                Control::Par(_) => {
                    out = Some(current);
                }
                // leaf
                Control::Invoke(_) | Control::Empty(_) | Control::Enable(_) => {
                    unreachable!("leaf nodes cannot be parents")
                }
            };

            // climb one level of the tree
            if out.is_none() {
                prior = current;
                current = ctx.primary[current].parent?;
            }
        }

        out.map(|x| node.new_retain_comp(x))
    }

    /// Attempts to get the next node for the given control point, if found
    /// it replaces the given node. Returns true if the node was found and
    /// replaced, returns false otherwise
    pub fn mutate_into_next(&mut self, ctx: &Context) -> bool {
        if let Some(next) = Self::get_next(self, ctx) {
            *self = next;
            true
        } else {
            false
        }
    }

    pub(super) fn should_reprocess(&self, ctx: &Context) -> bool {
        match &ctx.primary.control[self.control_node_idx].control {
            Control::Repeat(_)
            | Control::Empty(_)
            | Control::Seq(_)
            | Control::Par(_) => true,
            Control::Enable(_)
            | Control::If(_)
            | Control::While(_)
            | Control::Invoke(_) => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContinuousAssignments {
    pub comp: GlobalCellIdx,
    pub assigns: IndexRange<AssignmentIdx>,
}

/// The number of control points to preallocate for the program counter.
const CONTROL_POINT_PREALLOCATE: usize = 16;

/// The number of children that have yet to finish for a given par arm. I have
/// this a u16 at the moment which is hopefully fine? More than 65,535 parallel
/// children would be a lot.
pub type ChildCount = u16;

#[derive(Debug, Clone)]
pub struct WithEntry {
    pub group: CombGroupIdx,
    /// Whether or not a body has been executed. Only used by if statements
    pub entered: bool,
}

impl WithEntry {
    pub fn new(group: CombGroupIdx) -> Self {
        Self {
            group,
            entered: false,
        }
    }

    pub fn set_entered(&mut self) {
        self.entered = true;
    }
}

#[derive(Debug, Clone)]
pub struct ParEntry {
    child_count: ChildCount,
    finished_threads: SmallVec<[ThreadIdx; 4]>,
}

impl ParEntry {
    pub fn child_count_mut(&mut self) -> &mut ChildCount {
        &mut self.child_count
    }

    pub fn child_count(&self) -> u16 {
        self.child_count
    }
    pub fn add_finished_thread(&mut self, thread: ThreadIdx) {
        self.finished_threads.push(thread);
    }

    pub fn iter_finished_threads(&self) -> impl Iterator<Item = ThreadIdx> {
        self.finished_threads.iter().copied()
    }
}

impl TryFrom<usize> for ParEntry {
    type Error = std::num::TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(ParEntry {
            child_count: value.try_into()?,
            finished_threads: SmallVec::new(),
        })
    }
}

/// The program counter for the whole program execution. Wraps over a vector of
/// the active leaf statements for each component instance.
#[derive(Debug, Default, Clone)]
pub(crate) struct ProgramCounter {
    vec: Vec<ProgramPointer>,
    par_map: HashMap<ControlPoint, ParEntry>,
    continuous_assigns: Vec<ContinuousAssignments>,
    with_map: HashMap<ControlPoint, WithEntry>,
    repeat_map: HashMap<ControlPoint, u64>,
    just_finished_comps: Vec<(GlobalCellIdx, Option<ThreadIdx>)>,
    thread_memoizer: HashMap<(GlobalCellIdx, ThreadIdx, ControlIdx), ThreadIdx>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Active,
    Paused,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramPointer {
    thread: Option<ThreadIdx>,
    control_point: ControlPoint,
    status: ExecutionStatus,
}

impl ProgramPointer {
    pub fn new(
        thread: Option<ThreadIdx>,
        control_point: ControlPoint,
        status: ExecutionStatus,
    ) -> Self {
        Self {
            thread,
            control_point,
            status,
        }
    }

    pub fn new_active(thread: Option<ThreadIdx>, point: ControlPoint) -> Self {
        Self::new(thread, point, ExecutionStatus::Active)
    }

    pub fn new_paused(thread: Option<ThreadIdx>, point: ControlPoint) -> Self {
        Self::new(thread, point, ExecutionStatus::Paused)
    }

    pub fn thread(&self) -> Option<ThreadIdx> {
        self.thread
    }

    pub fn thread_mut(&mut self) -> &mut Option<ThreadIdx> {
        &mut self.thread
    }

    pub fn get_mut(&mut self) -> (&mut ControlPoint, &mut Option<ThreadIdx>) {
        (&mut self.control_point, &mut self.thread)
    }

    pub fn control_point(&self) -> &ControlPoint {
        &self.control_point
    }

    pub fn set_control_point(&mut self, new: ControlPoint) {
        self.control_point = new
    }

    pub fn status(&self) -> ExecutionStatus {
        self.status
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self.status, ExecutionStatus::Active)
    }

    /// Suspend execution of this portion of the program
    pub fn pause(&mut self) {
        self.status = ExecutionStatus::Paused
    }

    /// Unpause this portion of the program
    pub fn unpause(&mut self) {
        self.status = ExecutionStatus::Active
    }

    pub fn control_point_mut(&mut self) -> &mut ControlPoint {
        &mut self.control_point
    }

    pub fn control_idx(&self) -> ControlIdx {
        self.control_point.control_node_idx
    }

    /// Returns the cell of the component instance
    pub fn component(&self) -> GlobalCellIdx {
        self.control_point.comp
    }
}
// Type alias for fields that need to be extracted from the PC during execution
// for mutability reasons
pub type PcFields = (
    Vec<ProgramPointer>,
    HashMap<ControlPoint, ParEntry>,
    HashMap<ControlPoint, WithEntry>,
    HashMap<ControlPoint, u64>,
);

pub type PcMaps<'a> = (
    &'a mut HashMap<ControlPoint, ParEntry>,
    &'a mut HashMap<ControlPoint, WithEntry>,
    &'a mut HashMap<ControlPoint, u64>,
);

impl ProgramCounter {
    pub(crate) fn new_empty() -> Self {
        Self {
            vec: Vec::with_capacity(CONTROL_POINT_PREALLOCATE),
            par_map: HashMap::new(),
            continuous_assigns: Vec::new(),
            with_map: HashMap::new(),
            repeat_map: HashMap::new(),
            just_finished_comps: Vec::new(),
            thread_memoizer: HashMap::new(),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ProgramPointer> {
        self.vec.iter()
    }

    pub fn node_slice(&self) -> &[ProgramPointer] {
        &self.vec
    }

    pub fn vec_mut(&mut self) -> &mut Vec<ProgramPointer> {
        &mut self.vec
    }

    pub fn _par_map_mut(&mut self) -> &mut HashMap<ControlPoint, ParEntry> {
        &mut self.par_map
    }

    pub fn _par_map(&self) -> &HashMap<ControlPoint, ParEntry> {
        &self.par_map
    }

    pub fn take_fields(&mut self) -> PcFields {
        (
            std::mem::take(&mut self.vec),
            std::mem::take(&mut self.par_map),
            std::mem::take(&mut self.with_map),
            std::mem::take(&mut self.repeat_map),
        )
    }

    pub fn restore_fields(&mut self, fields: PcFields) {
        let (vec, par_map, with_map, repeat_map) = fields;
        self.vec = vec;
        self.par_map = par_map;
        self.with_map = with_map;
        self.repeat_map = repeat_map;
    }

    pub(crate) fn push_continuous_assigns(
        &mut self,
        comp: GlobalCellIdx,
        assigns: IndexRange<AssignmentIdx>,
    ) {
        let assigns = ContinuousAssignments { comp, assigns };
        self.continuous_assigns.push(assigns)
    }

    pub(crate) fn continuous_assigns(&self) -> &[ContinuousAssignments] {
        &self.continuous_assigns
    }

    pub(crate) fn with_map(&self) -> &HashMap<ControlPoint, WithEntry> {
        &self.with_map
    }

    pub fn set_finished_comp(
        &mut self,
        comp: GlobalCellIdx,
        thread: Option<ThreadIdx>,
    ) {
        self.just_finished_comps.push((comp, thread))
    }

    pub fn finished_comps(&self) -> &[(GlobalCellIdx, Option<ThreadIdx>)] {
        &self.just_finished_comps
    }

    pub fn clear_finished_comps(&mut self) {
        self.just_finished_comps.clear()
    }

    pub fn lookup_thread(
        &mut self,
        comp: GlobalCellIdx,
        thread: ThreadIdx,
        control: ControlIdx,
    ) -> Entry<(GlobalCellIdx, ThreadIdx, ControlIdx), ThreadIdx> {
        self.thread_memoizer.entry((comp, thread, control))
    }
}

impl<'a> IntoIterator for &'a ProgramCounter {
    type Item = &'a ProgramPointer;

    type IntoIter = std::slice::Iter<'a, ProgramPointer>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
