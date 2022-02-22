// this can be a copy type because it's just a bunch of bools
#[derive(Debug, Default, Clone, Copy)]
/// Configuration struct which controls runtime behavior
pub struct Config {
    /// enables/disables "sloppy" interpretation which returns 0 for invalid indicies
    /// rather than erroring
    pub allow_invalid_memory_access: bool,
    /// upgrades overflow/underflow warnings into errors
    pub error_on_overflow: bool,
    /// permits "sloppy" interpretation with parallel blocks
    pub allow_par_conflicts: bool,
    /// suppresses warnings
    pub quiet: bool,
}

#[derive(Default)]
pub struct ConfigBuilder {
    allow_invalid_memory_access: Option<bool>,
    error_on_overflow: Option<bool>,
    allow_par_conflicts: Option<bool>,
    quiet: Option<bool>,
}

impl ConfigBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quiet(mut self, value: bool) -> Self {
        self.quiet = Some(value);
        self
    }

    pub fn allow_invalid_memory_access(mut self, value: bool) -> Self {
        self.allow_invalid_memory_access = Some(value);
        self
    }

    pub fn error_on_overflow(mut self, value: bool) -> Self {
        self.error_on_overflow = Some(value);
        self
    }

    pub fn allow_par_conflicts(mut self, value: bool) -> Self {
        self.allow_par_conflicts = Some(value);
        self
    }

    pub fn build(self) -> Config {
        Config {
            allow_par_conflicts: self.allow_par_conflicts.unwrap_or_default(),
            error_on_overflow: self.error_on_overflow.unwrap_or_default(),
            quiet: self.quiet.unwrap_or_default(),
            allow_invalid_memory_access: self
                .allow_invalid_memory_access
                .unwrap_or_default(),
        }
    }
}
