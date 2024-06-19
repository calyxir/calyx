use calyx_pass_explorer::{
    cli::ParseArgs,
    pass_explorer::{Breakpoint, PassExplorer},
    tui::PassExplorerTUI,
    util,
};
use crossterm::style::Stylize;
use std::{path::PathBuf, process::exit};
use tempdir::TempDir;

fn fail<F>(message: &str, explain: F)
where
    F: FnOnce(),
{
    println!("{}: {}", "error".red().bold(), message);
    println!();
    explain();
    println!();
    println!("Run calyx-pass --help for more information.");
    exit(1);
}

#[allow(clippy::write_literal)]
#[allow(clippy::needless_range_loop)]
fn main() -> std::io::Result<()> {
    let mut args: ParseArgs = argh::from_env();

    if args.version {
        println!("calyx-pass v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("Features (v{}):", env!("CARGO_PKG_VERSION"));
        println!(" - View pass transformations");
        println!(" - Apply or skip passes");
        println!(" - Focus a component");
        println!(" - Set breakpoints");
        println!();
        println!("See more at https://github.com/calyxir/calyx/blob/calyx-pass/tools/calyx-pass/README.md");
        return Ok(());
    }

    // If the user provided no --calyx-exec (or passed in an empty string),
    // then we first try to obtain the location via fud, and otherwise default
    // to target/debug/calyx
    if args.calyx_exec.is_empty() {
        args.calyx_exec = "target/debug/calyx".into();

        if let Ok(calyx_exec_rel) = util::capture_command_stdout(
            "fud",
            &["config", "stages.calyx.exec"],
            true,
        ) {
            args.calyx_exec = calyx_exec_rel.trim().into();
        }
    }

    if !args.disable.is_empty() && args.breakpoint.is_none() {
        fail("Invalid command line flags.", || {
            println!("Using the disable pass option (`-d`) requires a breakpoint to be set. You can set one with `-b`.");
        });
    }

    assert!(!args.calyx_exec.is_empty(), "We just assigned it a non-empty value if it was empty (unless fud somehow set the calyx executable as empty...");

    if util::capture_command_stdout(&args.calyx_exec, &["--version"], true)
        .is_err()
    {
        fail("Failed to determine or repair calyx executable path automatically.", || {
                println!("{}", "Here's how to fix it:".bold());
                println!("Option 1. Setup your fud config so that 'stages.calyx.exec' yields a valid path to the calyx executable");
                println!("Option 2. Determine the path manually and pass it to the `-e` or `--calyx-exec` option");
                println!("Option 3. Run this tool from the repository directory after calling `cargo build`");
            });
    }

    if args.input_file.is_none() {
        fail("Invalid command line arguments.", || {
            println!("You must pass a single calyx program as input. However, when the version is requested through `--version`, this input file is not required and will be ignored.");
        });
    }

    // use . for tmpdir for debugging, eventually just use TempDir::new
    let temp_dir = TempDir::new(".calyx-pass")?;
    let pass_explorer = PassExplorer::new(
        temp_dir,
        args.calyx_exec,
        args.breakpoint
            .map(|pass| Breakpoint::from(pass, args.disable)),
        args.pass_alias,
        PathBuf::from(
            args.input_file.expect("No input file passed as required"),
        ),
    )?;

    let mut stdout = std::io::stdout();
    let mut tui =
        PassExplorerTUI::from(&mut stdout, pass_explorer, args.component)?;
    tui.run()?;

    Ok(())
}
