use std::{num::NonZeroU32, ops::Index};

use cider_idx::{impl_index_nonzero, maps::IndexedMap};

use super::environment::clock::ClockIdx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadIdx(NonZeroU32);
impl_index_nonzero!(ThreadIdx);

#[derive(Debug, Clone)]
pub struct ThreadInfo {
    parent: Option<ThreadIdx>,
    clock_id: ClockIdx,
}

impl ThreadInfo {
    pub fn parent(&self) -> Option<ThreadIdx> {
        self.parent
    }

    pub fn clock_id(&self) -> ClockIdx {
        self.clock_id
    }
}

#[derive(Debug, Clone)]
pub struct ThreadMap {
    map: IndexedMap<ThreadIdx, ThreadInfo>,
}

impl ThreadMap {
    pub fn new(root_clock: ClockIdx, continuous_clock: ClockIdx) -> Self {
        let mut map = IndexedMap::new();
        map.push(ThreadInfo {
            parent: None,
            clock_id: root_clock,
        });
        map.push(ThreadInfo {
            parent: None,
            clock_id: continuous_clock,
        });
        Self { map }
    }

    pub fn root_thread() -> ThreadIdx {
        ThreadIdx::from(0)
    }

    pub fn continuous_thread() -> ThreadIdx {
        ThreadIdx::from(1)
    }

    /// Lookup the clock associated with the given thread id. Returns `None` if
    /// the thread id is invalid.
    pub fn get_clock_id(&self, thread_id: &ThreadIdx) -> Option<ClockIdx> {
        self.map.get(*thread_id).map(|x| x.clock_id)
    }

    /// Lookup the clock associated with the given thread id. Panics if the
    /// thread id is invalid.
    pub fn unwrap_clock_id(&self, thread_id: ThreadIdx) -> ClockIdx {
        self.map.get(thread_id).unwrap().clock_id
    }

    /// Create a new thread with the given parent and clock id. Returns the new
    /// thread id.
    pub fn spawn(
        &mut self,
        parent: ThreadIdx,
        clock_id: ClockIdx,
    ) -> ThreadIdx {
        self.map.push(ThreadInfo {
            parent: Some(parent),
            clock_id,
        })
    }
}

impl Index<ThreadIdx> for ThreadMap {
    type Output = ThreadInfo;

    fn index(&self, index: ThreadIdx) -> &Self::Output {
        &self.map[index]
    }
}
