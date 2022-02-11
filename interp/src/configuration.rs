use lazy_static::*;
use parking_lot::RwLock;

lazy_static! {
    /// Global configuration object which stores the current configuration options for
    /// simulation and debugging. It is behind a RW lock but is largely meant to be
    /// read-only after simulation has begun.
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

#[derive(Default)]
pub struct Config {
    /// enables/disables "sloppy" interpretation which returns 0 for invalid indicies
    /// rather than erroring
    pub allow_invalid_memory_access: bool,
    /// upgrades overflow/underflow warnings into errors
    pub error_on_overflow: bool,
    /// permits "sloppy" interpretation with parallel blocks
    pub allow_par_conflicts: bool,
    /// suppresses warning printing
    pub quiet: bool,
}
