use makemake::{emitter::Emitter, makefile::Makefile};

use super::Testbench;
use std::{fs, io::Write, path::Path};

/// v1.8.1 cocotb
pub struct CocoTB;

fn filestem(path_str: &str) -> &str {
    let path = Path::new(path_str);
    path.file_stem()
        .expect("invalid filename")
        .to_str()
        .expect("invalid unicode")
}

impl Testbench for CocoTB {
    fn run(
        &self,
        input: String,
        tests: &[String],
        work_dir: tempdir::TempDir,
    ) -> super::TestbenchResult {
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
            makefile.comment(
            "include cocotb's make rules to take care of the simulator setup",
        );
            makefile.include("$(shell cocotb-config --makefiles)/Makefile.sim");

            let mut makefile_path = work_dir.path().to_path_buf();
            makefile_path.push("Makefile");
            fs::write(makefile_path, makefile.build())?;

            let output = std::process::Command::new("make")
                .current_dir(work_dir.path())
                .output()?;
            std::io::stdout().write_all(&output.stdout)?;
            std::io::stderr().write_all(&output.stderr)?;
        }

        Ok(())
    }
}
