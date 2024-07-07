use std::{
    io::{self, Write},
    process::Command,
};

use tb::{
    config::{Config, ConfigVarValidator},
    declare_plugin,
    error::{LocalError, LocalResult},
    plugin::Plugin,
    semver, tempdir,
};

mod config_keys {
    pub const EXE: &str = "exe";
    pub const CFLAGS: &str = "cflags";
    pub const TOP: &str = "top";
    pub const USE_SV: &str = "use-sv";
}

#[derive(Default)]
pub struct Verilator;

impl Verilator {
    fn create_build_files(
        &self,
        input: &str,
        test: &str,
        work_dir: &tempdir::TempDir,
        config: &Config,
    ) -> LocalResult<String> {
        let mut cmd =
            Command::new(config.get(config_keys::EXE)?.as_str().unwrap());
        cmd.current_dir(work_dir.path());
        cmd.args([
            "--cc", "--exe", "--build", "--timing", "-j", "0", "-Wall", input,
            test,
        ]);
        cmd.args([
            "--top-module",
            config.get(config_keys::TOP)?.as_str().unwrap(),
        ]);

        let cflags = config.get(config_keys::CFLAGS)?;
        let cflags = cflags.as_str().unwrap();
        cmd.args(["-CFLAGS", if cflags.is_empty() { "\"\"" } else { cflags }]);
        if config.get(config_keys::USE_SV)?.as_str().unwrap() == "true" {
            cmd.arg("-sv");
        }

        let output = cmd.output()?;
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;

        Ok(format!(
            "obj_dir/V{}",
            config.get(config_keys::TOP)?.as_str().unwrap()
        ))
    }

    fn execute_harness(
        &self,
        executable: String,
        work_dir: &tempdir::TempDir,
    ) -> LocalResult<()> {
        let output = Command::new(executable)
            .current_dir(work_dir.path())
            .output()?;
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;

        Ok(())
    }
}

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

        config.require(
            config_keys::CFLAGS,
            Some(""),
            "passed via -CFLAGS",
            ConfigVarValidator::default(),
        );

        config.require(
            config_keys::TOP,
            Some("main"),
            "name of top-level module",
            ConfigVarValidator::default(),
        );

        config.require(
            config_keys::USE_SV,
            Some("true"),
            "whether the input is SystemVerilog",
            ConfigVarValidator::when(|value| {
                value
                    .as_str()
                    .filter(|value| ["true", "false"].contains(value))
                    .ok_or(LocalError::other("must be true or false"))
                    .map(|_| ())
            }),
        );

        Ok(())
    }

    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: tempdir::TempDir,
        config: &Config,
    ) -> LocalResult<()> {
        for test in tests {
            let exec =
                self.create_build_files(&input, test, &work_dir, config)?;
            self.execute_harness(exec, &work_dir)?;
        }
        Ok(())
    }
}

declare_plugin!(Verilator, Verilator::default);
