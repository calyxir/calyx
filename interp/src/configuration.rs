use lazy_static::*;
use std::sync::RwLock;

lazy_static! {
    pub static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}
pub struct Config {
    /// enables/disables "sloppy" interpretation which returns 0 for invalid indicies
    /// rather than erroring
    pub allow_invalid_memory_access: bool,
    /// upgrades overflow/underflow warnings into errors
    pub error_on_overflow: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            allow_invalid_memory_access: false,
            error_on_overflow: false,
        }
    }
}
