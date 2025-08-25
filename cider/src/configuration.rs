use argh::FromArgValue;
use bon::{Builder, bon};

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
#[derive(Debug, Default, Clone, Copy)]
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
    /// Color Config
    pub color_config: ColorConfig,
    /// Whether to step through multiple program nodes in a single step
    pub allow_multistep: bool,
    /// disable memoization
    pub disable_memo: bool,
}
#[bon]
impl RuntimeConfig {
    #[builder]
    pub fn new(
        check_data_race: bool,
        debug_logging: bool,
        quiet: bool,
        allow_invalid_memory_access: bool,
        error_on_overflow: bool,
        undef_guard_check: bool,
        color_config: ColorConfig,
        allow_multistep: bool,
        disable_memo: bool,
    ) -> Self {
        let out = Self {
            check_data_race,
            debug_logging,
            quiet,
            allow_invalid_memory_access,
            error_on_overflow,
            undef_guard_check,
            color_config,
            allow_multistep,
            disable_memo,
        };

        out.configure_color_setting();
        out
    }

    pub fn get_logging_config(&self) -> LoggingConfig {
        LoggingConfig {
            quiet: self.quiet,
            debug_logging: self.debug_logging,
            color_config: self.color_config,
        }
    }

    fn configure_color_setting(&self) {
        text_utils::force_color(self.color_config);
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
    /// How to configure color for the logger
    pub color_config: ColorConfig,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ColorConfig {
    /// Force use of color output.
    #[default]
    On,
    /// Force no color output.
    Off,
    /// Infer color support
    Auto,
}

impl FromArgValue for ColorConfig {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "on" => Ok(ColorConfig::On),
            "false" | "0" | "off" => Ok(ColorConfig::Off),
            "infer" | "auto " => Ok(ColorConfig::Auto),
            _ => Err(format!(
                "Invalid color configuration: '{value}'. Expected 'on', 'off', or 'auto'."
            )),
        }
    }
}
