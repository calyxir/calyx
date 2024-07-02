use super::Testbench;
use crate::{
    config::{Config, ConfigVarValidator},
    error::LocalResult,
};

mod config_keys {
    pub const EXE: &str = "verilator-exe";
}

pub struct Verilator;

impl Testbench for Verilator {
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
