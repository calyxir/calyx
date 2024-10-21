use bon::Builder;

// this can be a copy type because it's just a bunch of bools
#[derive(Debug, Default, Clone, Copy, Builder)]
/// Configuration struct which controls runtime behavior
pub struct Config {
    /// suppresses warnings
    pub quiet: bool,
    /// dump registers as single entry memories
    pub dump_registers: bool,
    /// dumps all memories rather than just external ones
    pub dump_all_memories: bool,
}

/// Configuration struct containing options affecting the simulation time
/// decisions.
#[derive(Debug, Default, Clone, Copy, Builder)]
pub struct RuntimeConfig {
    /// enables data race checking
    pub check_data_race: bool,
    /// enables debug logging
    pub debug_logging: bool,
    /// enables/disables "sloppy" interpretation which returns 0 for invalid indices
    /// rather than erroring. (Currently defunct)
    pub allow_invalid_memory_access: bool,
    /// upgrades overflow/underflow warnings into errors (currently defunct)
    pub error_on_overflow: bool,
}
