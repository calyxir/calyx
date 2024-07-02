use tb::{
    config::{Config, ConfigVarValidator},
    declare_plugin,
    error::LocalResult,
    plugin::Plugin,
    semver, tempdir,
};

mod config_keys {
    pub const EXE: &str = "verilator-exe";
}

#[derive(Default)]
pub struct Verilator;

impl Plugin for Verilator {
    fn name(&self) -> &'static str {
        "verilator"
    }

    fn version(&self) -> semver::Version {
        semver::Version::new(0, 0, 0)
    }

    fn setup(&self, config: &mut Config) -> LocalResult<()> {
        config.require(
            config_keys::EXE,
            Some("verilator"),
            "path to verilator executable",
            ConfigVarValidator::default(),
        );
        Ok(())
    }

    fn run(
        &self,
        _input: String,
        _tests: &[String],
        _work_dir: tempdir::TempDir,
        _config: &Config,
    ) -> LocalResult<()> {
        todo!("verilator not yet impl")
    }
}

declare_plugin!(Verilator, Verilator::default);
