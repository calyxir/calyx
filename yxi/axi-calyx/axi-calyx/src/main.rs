use axi_calyx::{axi_gen::AXIGenerator, cli};
use colored::Colorize;
use log::LevelFilter;
use std::fs::{self, File};
use std::io::stdout;
use std::{io::stderr, io::Write, process::exit};

/// Whether the input file must end with `.yxi`.
const ENFORCE_EXTENSION: bool = false;

fn fail(msg: &str) -> std::io::Result<()> {
    writeln!(&mut stderr(), "{}: {}", "error".bright_red(), msg)?;
    exit(1);
}

fn main() -> std::io::Result<()> {
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .init();

    let args: cli::ParseArgs = argh::from_env();

    log::set_max_level(if args.quiet {
        LevelFilter::Off
    } else {
        LevelFilter::Info
    });

    if ENFORCE_EXTENSION && !args.input_file.ends_with(".yxi") {
        fail("Input file must end with '.yxi'.")?;
    }

    if fs::read_dir(args.lib_path.clone()).is_err() {
        fail(&format!(
            "'{}' is not a valid library path",
            args.lib_path.to_str().expect("argh parsed an invalid path")
        ))?;
    }

    log::info!("Loading YXI file from '{}'", args.input_file);
    let yxi_file = File::open(args.input_file)?;
    let yxi: yxi::ProgramInterface = serde_json::from_reader(yxi_file)?;

    let axi_gen = AXIGenerator::parse(yxi)
        .map_err(|err| fail(&format!("Failed to parse YXI file: {}", err)))
        .expect("`error` should have terminated the program");

    log::info!("Loaded YXI file for toplevel `{}`", axi_gen.yxi().toplevel);

    let ctx = axi_gen
        .build(args.lib_path)
        .map_err(|err| fail(&format!("{:?}", err)))
        .expect("We just handled error and exited program");
    calyx_ir::Printer::write_context(&ctx, true, &mut stdout())?;

    Ok(())
}
