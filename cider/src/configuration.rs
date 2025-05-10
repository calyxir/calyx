use bon::Builder;

use crate::flatten::text_utils;

// this can be a copy type because it's just a bunch of bools
#[derive(Debug, Default, Clone, Copy, Builder)]
/// Configuration struct which controls runtime behavior
pub struct Config {
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
    /// suppresses warnings
    pub quiet: bool,
    /// enables/disables "sloppy" interpretation which returns 0 for invalid indices
    /// rather than erroring. (Currently defunct)
    pub allow_invalid_memory_access: bool,
    /// upgrades overflow/underflow warnings into errors (currently defunct)
    pub error_on_overflow: bool,
    /// Check undefined guards
    pub undef_guard_check: bool,
}

impl RuntimeConfig {
    pub fn get_logging_config(&self) -> LoggingConfig {
        LoggingConfig {
            quiet: self.quiet,
            debug_logging: self.debug_logging,
        }
    }

    pub fn set_force_color(self, force_color: bool) {
        text_utils::force_color(force_color);
    }
}

/// Configuration struct describing what settings a logger should be created
/// with.
pub struct LoggingConfig {
    /// Whether or not to silence non-error messages. Will be overridden by
    /// `debug_logging` if set to true.
    pub quiet: bool,
    /// Whether or not to enable debug logging. If set to true, will override
    /// `quiet`.
    pub debug_logging: bool,
}
