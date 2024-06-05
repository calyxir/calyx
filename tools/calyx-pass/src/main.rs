use calyx_pass::{cli::ParseArgs, pass_explorer::PassExplorer, util};
use colored::Colorize;
use console::Term;
use std::{io::Write, path::PathBuf};
use tempdir::TempDir;

const SAVE_SCREEN: &str = "\x1b[?47h";
const RESTORE_SCREEN: &str = "\x1b[?47l";

fn main() -> std::io::Result<()> {
    let mut args: ParseArgs = argh::from_env();

    // If the user provided no --calyx-exec (or passed in an empty string),
    // then we first try to obtain the location via fud, and otherwise default
    // to target/debug/calyx
    if args.calyx_exec.is_empty() {
        args.calyx_exec = "target/debug/calyx".into();
        if let Ok(global_root) = util::capture_command_stdout(
            "fud",
            &["config", "global.root"],
            true,
        ) {
            if let Ok(calyx_exec_rel) = util::capture_command_stdout(
                "fud",
                &["config", "stages.calyx.exec"],
                true,
            ) {
                let mut path = PathBuf::new();
                path.push(global_root.trim());
                path.push(calyx_exec_rel.trim());
                args.calyx_exec = path.to_str().unwrap().into();
            }
        }
    }

    assert!(!args.calyx_exec.is_empty());

    if args.breakpoint.is_some() {
        println!("warning: -b/--break is currently WIP");
    }

    // use . for tmpdir for debugging, eventually just use TempDir::new
    let temp_dir = TempDir::new_in(".", ".calyx-pass")?;
    let mut pass_explorer = PassExplorer::new(
        temp_dir,
        args.calyx_exec,
        args.breakpoint,
        args.pass_alias,
        PathBuf::from(args.input_file),
    )?;

    const QUIT: char = 'q';
    const ACCEPT: char = 'a';

    // https://stackoverflow.com/a/55881770
    let mut term_stdout = Term::stdout();

    writeln!(term_stdout, "{}", SAVE_SCREEN)?;

    loop {
        // I know of no other way to clear the scrollback buffer
        // util::capture_command_stdout("clear", &[], true)?;
        writeln!(term_stdout, "\x1bc")?;

        writeln!(term_stdout, "{}", "Calyx Pass Explorer".underline())?;
        writeln!(
            term_stdout,
            " - usage: {} {}, {} {}",
            ACCEPT.to_string().bright_green(),
            "accept".green(),
            QUIT.to_string().bright_red(),
            "quit".red()
        )?;
        if let Some(last_pass) = pass_explorer.last_pass() {
            writeln!(term_stdout, " - last: {}", last_pass.bold())?;
        }
        if let Some(inc_pass) = pass_explorer.incoming_pass() {
            writeln!(term_stdout, " - incoming: {}", inc_pass.bold())?;
        }

        if let Some(review) = pass_explorer.review(args.component.clone())? {
            writeln!(term_stdout, "{}", review)?;
        }

        term_stdout.flush()?;
        // term_stdout.flush()?;

        match term_stdout.read_char()? {
            QUIT => break,
            ACCEPT => pass_explorer.accept()?,
            _ => (),
        }

        if pass_explorer.incoming_pass().is_none() {
            break;
        }
    }

    writeln!(term_stdout, "{}", RESTORE_SCREEN)?;

    Ok(())
}
