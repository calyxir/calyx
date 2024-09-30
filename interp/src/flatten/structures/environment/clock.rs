use std::{
    cmp::{max, Ordering},
    collections::HashMap,
    hash::Hash,
};

use crate::flatten::structures::{
    index_trait::impl_index, indexed_map::IndexedMap, thread::ThreadIdx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockIdx(u32);
impl_index!(ClockIdx);

use itertools::Itertools;
use thiserror::Error;

pub type ClockMap = IndexedMap<ClockIdx, VectorClock<ThreadIdx>>;

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
    pub fn join(first: &Self, second: &Self) -> Self {
        // might be better to use an iterator instead?
        let mut merged = first.clone();
        merged.sync(second);
        merged
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

        current_answer
    }
}

#[derive(Debug, Clone)]
pub struct ValueWithClock {
    pub value: baa::BitVecValue,
    pub write_clock: ClockIdx,
    pub read_clock: ClockIdx,
}

impl ValueWithClock {
    pub fn write(
        &mut self,
        writing_clock: ClockIdx,
        value: baa::BitVecValue,
        clocks: &mut ClockMap,
    ) -> Result<(), ClockError> {
        if clocks[writing_clock] >= clocks[self.write_clock]
            && clocks[writing_clock] >= clocks[self.read_clock]
        {
            self.value = value;
            clocks[self.write_clock] = clocks[writing_clock].clone();
            Ok(())
        } else if clocks[writing_clock]
            .partial_cmp(&clocks[self.read_clock])
            .is_none()
        {
            Err(ClockError::ReadWrite)
        } else if clocks[writing_clock]
            .partial_cmp(&clocks[self.write_clock])
            .is_none()
        {
            Err(ClockError::WriteWrite)
        } else {
            panic!("something weird happened. TODO griffin: Sort this out")
        }
    }

    pub fn read(
        &self,
        reading_clock: ClockIdx,
        clocks: &mut ClockMap,
    ) -> Result<(), ClockError> {
        if clocks[reading_clock] >= clocks[self.write_clock] {
            // TODO griffin: this is doing extra allocation. Probably would be
            // better to mutate the self.read_clock field directly but that
            // would require using some std::mem::take or otherwise getting
            // tricky (split_at_mut) to avoid issues with borrowing
            clocks[self.read_clock] = VectorClock::join(
                &clocks[self.read_clock],
                &clocks[reading_clock],
            );
            Ok(())
        } else if clocks[reading_clock]
            .partial_cmp(&clocks[self.write_clock])
            .is_none()
        {
            Err(ClockError::ReadWrite)
        } else {
            // This implies that the read happens before the write which I think
            // shouldn't be possible but also I am not sure.
            panic!("something weird happened. TODO griffin: Sort this out")
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum ClockError {
    #[error("Concurrent read & write to the same register")]
    ReadWrite,
    #[error("Concurrent writes to the same register")]
    WriteWrite,
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
