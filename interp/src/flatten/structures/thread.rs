use std::num::NonZeroU32;

use super::{
    environment::clock::ClockIdx, index_trait::impl_index_nonzero,
    indexed_map::IndexedMap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadIdx(NonZeroU32);
impl_index_nonzero!(ThreadIdx);

#[derive(Debug)]
struct ThreadInfo {
    #[allow(dead_code)]
    parent: Option<ThreadIdx>,
    clock_id: ClockIdx,
}

#[derive(Debug)]
pub struct ThreadMap {
    map: IndexedMap<ThreadIdx, ThreadInfo>,
}

impl ThreadMap {
    pub fn new() -> Self {
        Self {
            map: IndexedMap::new(),
        }
    }

    /// Creates a new root thread with the given clock id. Returns the new
    /// thread id. This should probably be used only once?
    pub fn create_root(&mut self, clock_id: ClockIdx) -> ThreadIdx {
        self.map.push(ThreadInfo {
            parent: None,
            clock_id,
        })
    }

    /// Lookup the clock associated with the given thread id. Returns `None` if
    /// the thread id is invalid.
    pub fn get_clock_id(&self, thread_id: &ThreadIdx) -> Option<ClockIdx> {
        self.map.get(*thread_id).map(|x| x.clock_id)
    }

    /// Lookup the clock associated with the given thread id. Panics if the
    /// thread id is invalid.
    pub fn unwrap_clock_id(&self, thread_id: &ThreadIdx) -> ClockIdx {
        self.map.get(*thread_id).unwrap().clock_id
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

impl Default for ThreadMap {
    fn default() -> Self {
        Self::new()
    }
}
