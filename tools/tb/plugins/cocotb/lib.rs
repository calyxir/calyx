use makemake::{emitter::Emitter, makefile::Makefile};
use std::io::{self, Write};
use std::process::Command;
use std::{fs, path::Path};
use tb::declare_plugin;
use tb::error::LocalError;
use tb::{
    config::{Config, ConfigVarValidator},
    error::LocalResult,
    plugin::Plugin,
    semver, tempdir,
};

/// v1.8.1 cocotb
#[derive(Default)]
pub struct CocoTB;

mod config_keys {
    pub const EXE: &str = "cocotb-config.exe";
    pub const SIM: &str = "sim";
}

fn filestem(path_str: &str) -> &str {
    let path = Path::new(path_str);
    path.file_stem()
        .expect("invalid filename")
        .to_str()
        .expect("invalid unicode")
}

impl Plugin for CocoTB {
    fn name(&self) -> &'static str {
        "cocotb"
    }

    fn version(&self) -> semver::Version {
        semver::Version::new(0, 0, 0)
    }

    fn setup(&self, config: &mut Config) -> LocalResult<()> {
        config.require(
            config_keys::EXE,
            Some("cocotb-config"),
            "path to cocotb-config executable",
            ConfigVarValidator::when(|value| {
                if let Some(cmd) = value.as_str() {
                    let output = Command::new(cmd)
                        .arg("--version")
                        .output()
                        .map_err(LocalError::from).map_err(|_| LocalError::other(format!("{} is not the cocotb-config executable", cmd)))?;
                    let version = String::from_utf8(output.stdout)
                    .map_err(|_| LocalError::other(format!("{} is not the cocotb-config executable", cmd)))?;
                    if version.trim() != "1.8.1" {
                        Err(LocalError::other("cocotb-config must be version 1.8.1"))
                    } else {
                        Ok(())
                    }
                } else {
                    Err(LocalError::other(
                        "the cocotb-config executable path must be specified as a string",
                    ))
                }
            }),
        );

        config.require(
            config_keys::SIM,
            Some("icarus"),
            "cocotb simulator",
            ConfigVarValidator::when(|value| {
                if let Some(sim) = value.as_str() {
                    let simulators = [
                        "icarus",
                        "verilator",
                        "vcs",
                        "riviera",
                        "activehdl",
                        "questa",
                        "modelsim",
                        "ius",
                        "xcelium",
                        "ghdl",
                        "cvc",
                    ];
                    if simulators.contains(&sim) {
                        Ok(())
                    } else {
                        Err(LocalError::other("unsupported simulator: see https://docs.cocotb.org/en/stable/simulator_support.html for details"))
                    }
                } else {
                    Err(LocalError::other(
                        "the cocotb simulator must be a string",
                    ))
                }
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
            // copied from https://github.com/cocotb/cocotb/blob/v1.8.1/examples/doc_examples/quickstart/Makefile
            let mut makefile = Makefile::new();
            makefile.comment("This file is public domain, it can be freely copied without restrictions.");
            makefile.comment("SPDX-License-Identifier: CC0-1.0");
            makefile.newline();
            makefile.comment("Makefile");
            makefile.newline();
            makefile.comment("defaults");
            makefile.assign_without_overwrite("SIM", "icarus");
            makefile.assign_without_overwrite("TOPLEVEL_LANG", "verilog");
            makefile.append("VERILOG_SOURCES", &input);
            makefile.comment("use VHDL_SOURCES for VHDL files");
            makefile.newline();
            makefile.comment("TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file");
            makefile.assign("TOPLEVEL", filestem(&input));
            makefile.newline();
            makefile.comment("MODULE is the basename of the Python test file");
            makefile.assign("MODULE", filestem(test));
            makefile.newline();
            makefile.comment("include cocotb's make rules to take care of the simulator setup");
            makefile.include(format!(
                "$(shell {} --makefiles)/Makefile.sim",
                config.get(config_keys::EXE)?.as_str().unwrap()
            ));

            let mut makefile_path = work_dir.path().to_path_buf();
            makefile_path.push("Makefile");
            fs::write(makefile_path, makefile.build())?;

            let output =
                Command::new("make").current_dir(work_dir.path()).output()?;
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }

        Ok(())
    }
}

declare_plugin!(CocoTB, CocoTB::default);
