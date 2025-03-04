use std::{
    cmp::{Ordering, max},
    collections::HashMap,
    hash::Hash,
    num::NonZeroU32,
    ops::{Index, IndexMut},
};

use crate::flatten::{
    flat_ir::{
        base::{AssignmentIdx, AssignmentWinner, GlobalCellIdx},
        component::AssignmentDefinitionLocation,
        prelude::ControlIdx,
    },
    structures::{
        context::Context,
        thread::{ThreadIdx, ThreadMap},
    },
    text_utils::Color,
};

use baa::BitVecValue;
use cider_idx::{
    impl_index_nonzero,
    maps::{IndexedMap, SecondarySparseMap},
};
use itertools::Itertools;
use thiserror::Error;

use super::Environment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClockIdx(NonZeroU32);
impl_index_nonzero!(ClockIdx);

pub type ThreadClockPair = (ThreadIdx, ClockIdx);

#[derive(Debug, Clone)]
pub struct ClockPairInfo {
    /// The cell that this clock pair was generated for
    pub attached_cell: GlobalCellIdx,
    /// An optional entry number within the given cell. This is used for
    /// memories but not for registers
    pub entry_number: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct WriteInfo {
    pub thread: ThreadIdx,
    pub assignment: AssignmentWinner,
}

#[derive(Debug, Clone)]
pub struct ReadList {
    pub map: SecondarySparseMap<ThreadIdx, ReadInfo>,
}

impl ReadList {
    pub fn new() -> Self {
        Self {
            map: SecondarySparseMap::new(),
        }
    }

    pub fn get_last_read(&self, thread: ThreadIdx) -> Option<&ReadInfo> {
        self.map.get(thread)
    }
    pub fn set_last_read(&mut self, thread: ThreadIdx, info: ReadInfo) {
        self.map.insert_value(thread, info);
    }
}

impl Default for ReadList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ReadWriteInfo {
    Write(WriteInfo),
    Read(ReadList),
}

impl ReadWriteInfo {
    #[must_use]
    pub fn as_write(&self) -> Option<&WriteInfo> {
        if let Self::Write(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_write_mut(&mut self) -> Option<&mut WriteInfo> {
        if let Self::Write(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_read(&self) -> Option<&ReadList> {
        if let Self::Read(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_read_mut(&mut self) -> Option<&mut ReadList> {
        if let Self::Read(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn unwrap_read(&self) -> &ReadList {
        self.as_read().unwrap()
    }

    pub fn unwrap_read_mut(&mut self) -> &mut ReadList {
        self.as_read_mut().unwrap()
    }

    pub fn unwrap_write(&self) -> &WriteInfo {
        self.as_write().unwrap()
    }
    pub fn unwrap_write_mut(&mut self) -> &mut WriteInfo {
        self.as_write_mut().unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum ReadSource {
    /// Read comes from the right hand side of an assignment
    Assignment(AssignmentIdx),
    /// Read comes from the guard of a given assignment
    Guard(AssignmentIdx),
    /// Read comes from the evaluation of a conditional
    Conditional(ControlIdx),
}

#[derive(Debug, Clone)]
pub struct ReadInfo {
    pub source: ReadSource,
    pub cell: GlobalCellIdx,
}

impl ReadInfo {
    pub fn new(source: ReadSource, cell: GlobalCellIdx) -> Self {
        Self { source, cell }
    }

    pub fn add_thread(self, thread: ThreadIdx) -> ReadInfoWithThread {
        ReadInfoWithThread {
            read_info: self,
            thread,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReadInfoWithThread {
    read_info: ReadInfo,
    thread: ThreadIdx,
}

impl ReadInfoWithThread {
    pub fn format<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> String {
        let ctx = env.ctx();
        let read_string = match self.read_info.source {
            ReadSource::Assignment(assignment_idx) => {
                let (comp, assign_src) =
                    ctx.find_assignment_definition(assignment_idx);
                let location_str = format_assignment_location(
                    env,
                    comp,
                    assign_src,
                    self.read_info.cell,
                );
                format!(
                    "RHS in assignment {} in {}",
                    ctx.printer()
                        .print_assignment(comp, assignment_idx)
                        .stylize_assignment(),
                    location_str
                )
            }
            ReadSource::Guard(assignment_idx) => {
                let (comp, assign_src) =
                    ctx.find_assignment_definition(assignment_idx);
                let location_str = format_assignment_location(
                    env,
                    comp,
                    assign_src,
                    self.read_info.cell,
                );
                format!(
                    "guard of assignment {} in {}",
                    ctx.printer()
                        .print_assignment(comp, assignment_idx)
                        .stylize_assignment(),
                    location_str
                )
            }
            ReadSource::Conditional(control_idx) => {
                let comp = ctx.lookup_control_definition(control_idx);
                let name = env.get_full_name(self.read_info.cell);
                let name = name.stylize_name();
                format!(
                    "conditional evaluation in {name}: \n {}",
                    ctx.printer().format_control(comp, control_idx, 1)
                )
            }
        };

        format!("read in thread {:?} from {}", self.thread, read_string)
    }
}

impl WriteInfo {
    pub fn new(thread: ThreadIdx, assignment: AssignmentWinner) -> Self {
        Self { thread, assignment }
    }
    pub fn format<C: AsRef<Context> + Clone>(
        &self,
        env: &Environment<C>,
    ) -> String {
        match self.assignment {
            AssignmentWinner::Cell => todo!(),
            AssignmentWinner::Implicit => todo!(),
            AssignmentWinner::Assign(assignment_idx, global_cell_idx) => {
                let ctx = env.ctx();
                let comp_idx = env.cells[global_cell_idx].unwrap_comp().comp_id;
                let assign_def = ctx
                    .lookup_assignment_definition(assignment_idx, comp_idx)
                    .unwrap();

                let location_str = format_assignment_location(
                    env,
                    comp_idx,
                    assign_def,
                    global_cell_idx,
                );

                format!(
                    "write in thread {:?} from assignment {} in {}",
                    self.thread,
                    ctx.printer()
                        .print_assignment(comp_idx, assignment_idx)
                        .stylize_assignment(),
                    location_str
                )
            }
        }
    }
}

fn format_assignment_location<C: Clone + AsRef<Context>>(
    env: &Environment<C>,
    comp_idx: crate::flatten::flat_ir::prelude::ComponentIdx,
    assign_def: AssignmentDefinitionLocation,
    cell: GlobalCellIdx,
) -> String {
    match assign_def {
        AssignmentDefinitionLocation::CombGroup(comb_group_idx) => {
            format!(
                "comb group {}::{}",
                env.get_full_name(cell),
                env.ctx().lookup_name(comb_group_idx).stylize_name(),
            )
        }
        AssignmentDefinitionLocation::Group(group_idx) => {
            format!(
                "group {}::{}",
                env.get_full_name(cell),
                env.ctx().lookup_name(group_idx).stylize_name()
            )
        }
        AssignmentDefinitionLocation::ContinuousAssignment => {
            format!(
                "continuous logic in {}",
                env.get_full_name(cell).stylize_name()
            )
        }
        AssignmentDefinitionLocation::Invoke(control_idx) => {
            format!(
                "invoke statement: {}",
                env.ctx().printer().format_control(comp_idx, control_idx, 0)
            )
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ClockMap {
    clocks: IndexedMap<ClockIdx, VectorClock<ThreadIdx>>,
    reverse_map: HashMap<ClockPair, ClockPairInfo>,
    extra_info: SecondarySparseMap<ClockIdx, ReadWriteInfo>,
}

impl ClockMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// pushes a new clock into the map and returns its index
    pub fn new_clock(&mut self) -> ClockIdx {
        self.clocks.push(VectorClock::new())
    }

    pub fn new_clock_pair(&mut self) -> ClockPair {
        let read = self.new_clock();
        let write = self.new_clock();
        ClockPair::new(read, write)
    }

    pub fn insert_reverse_entry(
        &mut self,
        pair: ClockPair,
        cell: GlobalCellIdx,
        entry_number: Option<u32>,
    ) {
        self.reverse_map.insert(
            pair,
            ClockPairInfo {
                attached_cell: cell,
                entry_number,
            },
        );
    }

    pub fn lookup_cell(&self, pair: ClockPair) -> Option<&ClockPairInfo> {
        self.reverse_map.get(&pair)
    }

    /// Returns a new clock that is the clone of the given clock
    pub fn fork_clock(&mut self, parent: ClockIdx) -> ClockIdx {
        self.clocks.push(self.clocks[parent].clone())
    }

    pub fn log_write(&mut self, clock: ClockIdx, info: WriteInfo) {
        self.extra_info
            .insert_value(clock, ReadWriteInfo::Write(info));
    }
    pub fn get_logged_write(&mut self, clock: ClockIdx) -> Option<&WriteInfo> {
        self.extra_info.get(clock).map(|x| x.unwrap_write())
    }

    pub fn log_read(
        &mut self,
        reading_thread: ThreadIdx,
        clock: ClockIdx,
        read: ReadInfo,
    ) {
        let entry = if let Some(entries) = self.extra_info.get_mut(clock) {
            entries.unwrap_read_mut()
        } else {
            self.extra_info
                .insert_value(clock, ReadWriteInfo::Read(ReadList::new()));
            self.extra_info.get_mut(clock).unwrap().unwrap_read_mut()
        };
        entry.set_last_read(reading_thread, read);
    }

    pub fn get_logged_reads<'a, I>(
        &'a self,
        read_clock: ClockIdx,
        threads: I,
    ) -> impl Iterator<Item = ReadInfoWithThread> + 'a
    where
        I: IntoIterator<Item = ThreadIdx> + 'a,
    {
        let read_log = self.extra_info.get(read_clock).unwrap().unwrap_read();
        threads
            .into_iter()
            .map(|t| read_log.get_last_read(t).unwrap().clone().add_thread(t))
    }

    pub fn split_mut_indices(
        &mut self,
        idx1: ClockIdx,
        idx2: ClockIdx,
    ) -> Option<(&mut VectorClock<ThreadIdx>, &mut VectorClock<ThreadIdx>)>
    {
        self.clocks.split_mut_indices(idx1, idx2)
    }
}

impl Index<ClockIdx> for ClockMap {
    type Output = VectorClock<ThreadIdx>;
    fn index(&self, index: ClockIdx) -> &Self::Output {
        &self.clocks[index]
    }
}

impl IndexMut<ClockIdx> for ClockMap {
    fn index_mut(&mut self, index: ClockIdx) -> &mut Self::Output {
        &mut self.clocks[index]
    }
}

pub trait Counter: Default {
    /// Increment the counter, returning `None` if the counter overflowed.
    #[must_use]
    fn increment(&mut self) -> Option<()>;

    /// Increment the counter, panicking if the counter overflowed.
    fn increment_expect(&mut self) {
        self.increment().expect("counter overflowed");
    }
}

impl Counter for u8 {
    fn increment(&mut self) -> Option<()> {
        *self = self.checked_add(1)?;
        Some(())
    }
}

impl Counter for u16 {
    fn increment(&mut self) -> Option<()> {
        *self = self.checked_add(1)?;
        Some(())
    }
}

impl Counter for u32 {
    fn increment(&mut self) -> Option<()> {
        *self = self.checked_add(1)?;
        Some(())
    }
}

impl Counter for u64 {
    fn increment(&mut self) -> Option<()> {
        *self = self.checked_add(1)?;
        Some(())
    }
}

// I don't expect this to be used much, but it's here for completeness
impl Counter for u128 {
    fn increment(&mut self) -> Option<()> {
        *self = self.checked_add(1)?;
        Some(())
    }
}

/// If the clock map is provided, use it to create a new clock. Otherwise,
/// return the 0th clock idx.
pub fn new_clock_pair(
    clock_map: &mut Option<&mut ClockMap>,
    cell: GlobalCellIdx,
    entry_number: Option<u32>,
) -> ClockPair {
    if let Some(map) = clock_map {
        let pair = map.new_clock_pair();
        map.insert_reverse_entry(pair, cell, entry_number);
        pair
    } else {
        ClockPair::zero()
    }
}

/// A simple vector clock implementation.
///
/// Internally uses a [`HashMap`] to store the clock values. Keys which are not
/// present in the map are assumed to be the default value for the given counter
/// type, which is zero for the standard integer counters. This means that all
/// threads implicitly start at zero, rather than some bottom value.
#[derive(Debug, Clone)]
pub struct VectorClock<I, C = u32>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    // TODO: maybe use `ahash` instead
    map: HashMap<I, C>,
}

impl<I, C> std::ops::Index<&I> for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    type Output = C;

    fn index(&self, index: &I) -> &Self::Output {
        &self.map[index]
    }
}

impl<I, C> std::ops::Index<I> for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    type Output = C;

    fn index(&self, index: I) -> &Self::Output {
        &self.map[&index]
    }
}

impl<I, C> Eq for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
}

impl<I, C> PartialEq for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    fn eq(&self, other: &Self) -> bool {
        if let Some(c) = self.partial_cmp(other) {
            matches!(c, Ordering::Equal)
        } else {
            false
        }
    }
}

impl<I, C> FromIterator<(I, C)> for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    fn from_iter<T: IntoIterator<Item = (I, C)>>(iter: T) -> Self {
        Self {
            map: iter.into_iter().collect(),
        }
    }
}

impl<I, C> VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn new_incr(id: I) -> Self {
        let mut clock = Self::new();
        clock.increment(&id);
        clock
    }

    /// Increment the clock for the given id. Creates the id if it doesn't exist.
    ///
    /// # Panics
    /// Panics if the clock overflows.
    pub fn increment(&mut self, id: &I) {
        if let Some(counter) = self.map.get_mut(id) {
            counter.increment_expect();
        } else {
            self.map.insert(id.clone(), C::default());
            self.map.get_mut(id).unwrap().increment_expect();
        }
    }

    pub fn get(&self, id: &I) -> Option<&C> {
        self.map.get(id)
    }

    /// Takes two vector clocks and mutates the first such that it contains the
    /// maximum value for each local clock across both vector clocks.
    pub fn sync(&mut self, other: &Self) {
        for (id, counter) in other.map.iter() {
            let v = self.map.entry(id.clone()).or_default();
            *v = max(counter, v).clone();
        }
    }

    /// Takes two vector clocks and produces a new vector clock that contains
    /// the maximum value for each local clock across both vector clocks.
    #[inline]
    pub fn join(first: &Self, second: &Self) -> Self {
        // might be better to use an iterator instead?
        let mut merged = first.clone();
        merged.sync(second);
        merged
    }

    pub fn set_thread_clock(&mut self, thread_id: I, clock: C) {
        self.map.insert(thread_id, clock);
    }

    /// Returns an iterator over ids in the `other` vector clock which are
    /// strictly greater than in `self`
    pub fn get_strictly_greater<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = &'a I> + 'a {
        other.map.iter().filter_map(|(key, other_count)| {
            if let Some(count) = self.get(key) {
                if other_count > count { Some(key) } else { None }
            } else {
                Some(key)
            }
        })
    }
}

impl<I, C> Default for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, C> PartialOrd for VectorClock<I, C>
where
    I: Hash + Eq + Clone,
    C: Ord + Clone + Counter,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // there's probably a better way to do this but it'll suffice for now
        // not sure if it's better to do extra redundant comparisons or incur
        // the cost of the `unique` call. Something to investigate in the future
        let iter = self.map.keys().chain(other.map.keys()).unique().map(|id| {
            match (self.get(id), other.get(id)) {
                (None, Some(count_other)) => C::default().cmp(count_other),
                (Some(count_self), None) => count_self.cmp(&C::default()),
                (Some(count_self), Some(count_other)) => {
                    count_self.cmp(count_other)
                }
                (None, None) => unreachable!(),
            }
        });

        let mut current_answer = None;
        for cmp in iter {
            if let Some(current_answer) = current_answer.as_mut() {
                match (&current_answer, cmp) {
                    // Incomparable case
                    (Ordering::Less, Ordering::Greater)
                    | (Ordering::Greater, Ordering::Less) => {
                        return None;
                    }
                    (Ordering::Equal, Ordering::Less) => {
                        *current_answer = Ordering::Less;
                    }
                    (Ordering::Equal, Ordering::Greater) => {
                        *current_answer = Ordering::Greater;
                    }
                    _ => {}
                }
            } else {
                current_answer = Some(cmp);
            }
        }

        // If we have an answer, return it. Otherwise, return `Equal` since the
        // `None` case can only happen if the maps are both empty. The
        // incomparable case exits early.
        if let Some(answer) = current_answer {
            Some(answer)
        } else {
            Some(Ordering::Equal)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValueWithClock {
    pub value: BitVecValue,
    pub clocks: ClockPair,
}

impl ValueWithClock {
    pub fn zero(width: u32, clocks: ClockPair) -> Self {
        Self {
            value: BitVecValue::zero(width),
            clocks,
        }
    }

    pub fn new(value: BitVecValue, clock_pair: ClockPair) -> Self {
        Self {
            value,
            clocks: clock_pair,
        }
    }
}

/// A struct containing the read and write clocks for a value. This is small
/// enough to be copied around easily
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct ClockPair {
    pub read_clock: ClockIdx,
    pub write_clock: ClockIdx,
}

impl ClockPair {
    /// Returns a new clock pair where both indices point to the zero clock.
    /// This should only be used as a placeholder entry for when clocks are not
    /// actually being tracked.
    pub fn zero() -> Self {
        Self {
            read_clock: ClockIdx::from(0),
            write_clock: ClockIdx::from(0),
        }
    }

    pub fn new(read_clock: ClockIdx, write_clock: ClockIdx) -> Self {
        Self {
            read_clock,
            write_clock,
        }
    }

    fn check_read(
        &self,
        (thread, reading_clock): ThreadClockPair,
        clock_map: &mut ClockMap,
    ) -> Result<(), ReadError> {
        if clock_map[reading_clock] >= clock_map[self.write_clock] {
            let v = clock_map[reading_clock][thread];
            clock_map[self.read_clock].set_thread_clock(thread, v);
            Ok(())
        } else if clock_map[reading_clock]
            .partial_cmp(&clock_map[self.write_clock])
            .is_none()
        {
            Err(ReadError)
        } else {
            // This implies that the read happens before the write which I think
            // shouldn't be possible
            unreachable!(
                "something weird happened. TODO griffin: Sort this out"
            )
        }
    }

    /// A wrapper method which checks the read and adds cell info on an error
    pub fn check_read_with_ascription(
        &self,
        (thread, reading_clock): ThreadClockPair,
        source: ReadSource,
        cell: GlobalCellIdx,
        clock_map: &mut ClockMap,
    ) -> Result<(), ClockError> {
        let res =
            self.check_read((thread, reading_clock), clock_map)
                .map_err(|_| ClockError::ReadAfterWrite {
                    write: clock_map
                        .get_logged_write(self.write_clock)
                        .unwrap()
                        .clone(),
                    read: ReadInfo::new(source.clone(), cell)
                        .add_thread(thread),
                });

        if res.is_ok() {
            clock_map.log_read(
                thread,
                self.read_clock,
                ReadInfo::new(source, cell),
            )
        }
        res
    }

    fn check_write(
        &self,
        writing_clock: ClockIdx,
        clock_map: &mut ClockMap,
    ) -> Result<(), WriteError> {
        if clock_map[writing_clock] >= clock_map[self.write_clock]
            && clock_map[writing_clock] >= clock_map[self.read_clock]
        {
            clock_map[self.write_clock] = clock_map[writing_clock].clone();
            Ok(())
        } else if clock_map[writing_clock] < clock_map[self.read_clock]
            || clock_map[writing_clock]
                .partial_cmp(&clock_map[self.read_clock])
                .is_none()
        {
            // dbg!(&clock_map[writing_clock], &clock_map[self.read_clock]);
            Err(WriteError::write_read(
                clock_map[writing_clock]
                    .get_strictly_greater(&clock_map[self.read_clock])
                    .copied()
                    .collect(),
            ))
        } else if clock_map[writing_clock]
            .partial_cmp(&clock_map[self.write_clock])
            .is_none()
        {
            Err(WriteError::WriteWrite)
        } else {
            // This implies the current write happened before the prior write
            // which I think shouldn't be possible
            unreachable!(
                "something weird happened. TODO griffin: Sort this out"
            )
        }
    }

    pub fn check_write_with_ascription(
        &self,
        thread: ThreadIdx,
        thread_map: &ThreadMap,
        clock_map: &mut ClockMap,
        winner: &AssignmentWinner,
    ) -> Result<(), ClockError> {
        let current_clock = thread_map.unwrap_clock_id(thread);
        let result = self.check_write(current_clock, clock_map);
        match result {
            Ok(val) => {
                clock_map.log_write(
                    self.write_clock,
                    WriteInfo::new(thread, winner.clone()),
                );
                Ok(val)
            }
            Err(e) => match e {
                WriteError::WriteWrite => Err(ClockError::WriteAfterWrite {
                    write1: clock_map
                        .get_logged_write(self.write_clock)
                        .unwrap()
                        .clone(),
                    write2: WriteInfo::new(thread, winner.clone()),
                }),
                WriteError::WriteRead(threads) => {
                    let reads = clock_map
                        .get_logged_reads(self.read_clock, threads)
                        .collect();

                    Err(ClockError::WriteAfterRead {
                        write: WriteInfo::new(thread, winner.clone()),
                        reads,
                    })
                }
            },
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum WriteError {
    #[error("concurrent writes to same value")]
    WriteWrite,
    #[error("concurrent write and read to same value")]
    WriteRead(Box<[ThreadIdx]>),
}

impl WriteError {
    fn write_read(threads: Box<[ThreadIdx]>) -> Self {
        assert!(!threads.is_empty());
        Self::WriteRead(threads)
    }
}

#[derive(Debug, Clone, Error)]
#[error("Concurrent read and write")]
/// An empty error struct for when a read fails. Since a read can only fail
/// because there was an earlier write which is concurrent with this read, no
/// additional information is needed.
pub struct ReadError;

#[derive(Debug, Clone, Error)]
pub enum ClockError {
    #[error(
        "Concurrent read & write to the same register/memory. This text should never be seen"
    )]
    ReadAfterWrite {
        write: WriteInfo,
        read: ReadInfoWithThread,
    },
    #[error(
        "Concurrent writes to the same register/memory. This text should never be seen"
    )]
    WriteAfterWrite {
        write1: WriteInfo,
        write2: WriteInfo,
    },
    #[error(
        "Concurrent writes to the same register/memory. This text should never be seen"
    )]
    WriteAfterRead {
        write: WriteInfo,
        reads: Box<[ReadInfoWithThread]>,
    },
}

#[derive(Debug, Clone, Error)]
#[error("encountered error {error}. This text should never be seen")]
pub struct ClockErrorWithCell {
    pub error: ClockError,
    pub cell: GlobalCellIdx,
    pub entry_number: Option<u32>,
}

impl ClockError {
    pub fn add_cell_info(
        self,
        cell: GlobalCellIdx,
        entry_number: Option<u32>,
    ) -> ClockErrorWithCell {
        ClockErrorWithCell {
            error: self,
            cell,
            entry_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let clock = VectorClock::<u32>::new_incr(1);
        let mut other = VectorClock::<u32>::new();

        assert!(clock >= other);

        other.increment(&2);

        assert!(clock.partial_cmp(&other).is_none());

        let merged = VectorClock::join(&clock, &other);

        assert!(merged >= clock);
        assert!(merged >= other);
        assert_eq!(VectorClock::<u32>::new(), VectorClock::<u32>::new());
    }

    // Adapted from VClock tests
    #[test]
    fn test_vector_clock_new() {
        let vc1 = VectorClock::<u32>::new_incr(17);
        assert_eq!(None, vc1.get(&0));
        assert_eq!(Some(1), vc1.get(&17).copied());

        let vc2 = VectorClock::<u32>::new_incr(17u32);
        assert_eq!(None, vc2.get(&0u32));
        assert_eq!(Some(1), vc2.get(&17u32).copied());

        let vc3 = VectorClock::<i64, u8>::new_incr(17i64);
        assert_eq!(None, vc3.get(&0i64));
        assert_eq!(Some(1u8), vc3.get(&17i64).copied());
    }

    #[test]
    fn test_vector_clock_increment() {
        let mut vc = VectorClock::<i16>::new();

        assert_eq!(None, vc.get(&0));
        assert_eq!(None, vc.get(&2));

        vc.increment(&0);
        assert_eq!(Some(1), vc.get(&0).copied());
        assert_eq!(None, vc.get(&2));

        vc.increment(&2);
        vc.increment(&0);

        assert_eq!(Some(2), vc.get(&0).copied());
        assert_eq!(Some(1), vc.get(&2).copied());

        vc.increment(&0);
        assert_eq!(Some(3), vc.get(&0).copied());
        assert_eq!(Some(1), vc.get(&2).copied());

        vc.increment(&1);
        assert_eq!(Some(3), vc.get(&0).copied());
        assert_eq!(Some(1), vc.get(&1).copied());
        assert_eq!(Some(1), vc.get(&2).copied());
    }

    #[test]
    fn test_empty_comparison() {
        let vc = VectorClock::<u32>::new();
        let vc2: VectorClock<u32> = [(12, 0), (10, 0)].into_iter().collect();
        let vc3: VectorClock<u32> = [(147, 0), (32, 0)].into_iter().collect();

        assert_eq!(vc, vc2);
        assert_eq!(vc2, vc3);
    }

    #[test]
    #[should_panic]
    fn test_overflow() {
        let mut vc = VectorClock::<u8, u8>::new();
        for _ in 0..257 {
            vc.increment(&0);
        }
    }
}
