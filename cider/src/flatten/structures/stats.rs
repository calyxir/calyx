use std::sync::{LazyLock, MutexGuard};

#[derive(Debug, Clone)]
pub struct DataRaceStatistics {
    /// the number of times a thread was spawned. Not the same as the number of
    /// unique thread IDs
    pub thread_spawn_count: u64,
    pub fork_count: u64,
    pub join_count: u64,
    pub read_count: u64,
    pub write_count: u64,
}

impl DataRaceStatistics {
    pub fn new() -> Self {
        Self {
            thread_spawn_count: 0,
            fork_count: 0,
            join_count: 0,
            read_count: 0,
            write_count: 0,
        }
    }

    pub fn incr_thread_spawn(&mut self) {
        self.thread_spawn_count += 1;
    }

    pub fn incr_fork_count(&mut self) {
        self.fork_count += 1;
    }

    pub fn incr_join_count(&mut self) {
        self.join_count += 1;
    }

    pub fn incr_read_count(&mut self) {
        self.read_count += 1;
    }

    pub fn incr_write_count(&mut self) {
        self.write_count += 1;
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

impl Default for DataRaceStatistics {
    fn default() -> Self {
        Self::new()
    }
}

static DR_STATS: LazyLock<std::sync::Mutex<DataRaceStatistics>> =
    LazyLock::new(|| DataRaceStatistics::new().into());

pub fn incr_thread_spawn() {
    DR_STATS
        .lock()
        .expect("stat lock poisoned")
        .incr_thread_spawn();
}

pub fn incr_fork_count() {
    DR_STATS
        .lock()
        .expect("stat lock poisoned")
        .incr_fork_count();
}

pub fn incr_join_count() {
    DR_STATS
        .lock()
        .expect("stat lock poisoned")
        .incr_join_count();
}

pub fn incr_read_count() {
    DR_STATS
        .lock()
        .expect("stat lock poisoned")
        .incr_read_count();
}

pub fn incr_write_count() {
    DR_STATS
        .lock()
        .expect("stat lock poisoned")
        .incr_write_count();
}

pub fn clear_stats() {
    DR_STATS.lock().expect("stat lock poisoned").clear()
}

pub fn get_stats() -> MutexGuard<'static, DataRaceStatistics> {
    DR_STATS.lock().expect("stat lock poisoned")
}
