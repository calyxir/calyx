use axi_calyx::{axi_gen::AXIGenerator, cli};
use bad_calyx_builder as calyx_builder;
use colored::Colorize;
use serde::Deserialize;
use serde_json::Result;
use std::fs::File;
use std::{io::stderr, io::Write, process::exit};

/// Whether the input file must end with `.yxi`.
const ENFORCE_EXTENSION: bool = false;

fn error(msg: &str) -> std::io::Result<()> {
    writeln!(&mut stderr(), "{}: {}", "error".bright_red(), msg)?;
    exit(1);
}

fn main() -> std::io::Result<()> {
    let args: cli::ParseArgs = argh::from_env();

    if ENFORCE_EXTENSION && !args.input_file.ends_with(".yxi") {
        error("Input file must end with '.yxi'.")?;
    }

    let yxi_file = File::open(args.input_file)?;
    let yxi: yxi::ProgramInterface = serde_json::from_reader(yxi_file)?;

    let axi_gen = AXIGenerator::parse(yxi)
        .map_err(|err| error(&format!("Failed to parse YXI file: {}", err)))
        .expect("`error` should have terminated the program");

    Ok(())
}
