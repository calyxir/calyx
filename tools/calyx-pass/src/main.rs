use calyx_pass::{
    cli::ParseArgs,
    pass_explorer::{PassApplicationStatus, PassExplorer},
    util
};
use colored::Colorize;
use console::Term;
use std::{
    cmp::{max, min},
    io::Write,
    path::PathBuf
};
use tempdir::TempDir;

/// Saves the terminal buffer.
///
/// Source: https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
const SAVE_SCREEN: &str = "\x1b[?47h";

/// Restores the terminal buffer. See [`SAVE_SCREEN`].
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
            true
        ) {
            if let Ok(calyx_exec_rel) = util::capture_command_stdout(
                "fud",
                &["config", "stages.calyx.exec"],
                true
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
        PathBuf::from(args.input_file)
    )?;

    const QUIT: char = 'q';
    const ACCEPT: char = 'a';
    const SKIP: char = 's';

    // https://stackoverflow.com/a/55881770
    let mut term_stdout = Term::stdout();

    writeln!(term_stdout, "{}", SAVE_SCREEN)?;
    term_stdout.hide_cursor()?;

    loop {
        // I know of no other way to clear the scrollback buffer
        // util::capture_command_stdout("clear", &[], true)?;
        writeln!(term_stdout, "\x1bc")?;

        writeln!(term_stdout, "{}", "Calyx Pass Explorer".underline())?;
        writeln!(
            term_stdout,
            "usage: {} {}, {} {}, {} {}",
            ACCEPT.to_string().bright_green(),
            "accept".green(),
            SKIP.to_string(),
            "skip",
            QUIT.to_string().bright_red(),
            "quit".red()
        )?;

        let current_pass_application = pass_explorer.current_pass_application();
        if let Some(incoming_pos) = current_pass_application
            .iter()
            .position(|(_, status)| *status == PassApplicationStatus::Incoming)
        {
            write!(term_stdout, "passes: ")?;
            let start_index = max(0, (incoming_pos as isize) - 3) as usize;
            if start_index > 0 {
                write!(term_stdout, "[{} more] ... ", start_index)?;
            }

            let length = min(5, current_pass_application.len() - start_index);
            for i in start_index..start_index + length {
                if i > start_index {
                    write!(term_stdout, ", ")?;
                }
                let (name, status) = &current_pass_application[i];
                let colored_name = match status {
                    PassApplicationStatus::Applied => name.green(),
                    PassApplicationStatus::Skipped => name.dimmed(),
                    PassApplicationStatus::Incoming => {
                        format!("[INCOMING] {}", name).yellow().bold()
                    }
                    PassApplicationStatus::Future => name.purple()
                };
                write!(term_stdout, "{}", colored_name)?;
            }

            let remaining_count =
                current_pass_application.len() - start_index - length;
            if remaining_count > 0 {
                write!(term_stdout, " ... [{} more]", remaining_count)?;
            }

            writeln!(term_stdout)?;
        }

        if let Some(review) = pass_explorer.review(args.component.clone())? {
            let rows = term_stdout.size().1;
            writeln!(term_stdout, "{}", "─".repeat(rows as usize).dimmed())?;
            write!(term_stdout, "{}", review)?;
        }

        match term_stdout.read_char()? {
            QUIT => break,
            ACCEPT => pass_explorer.advance(true)?,
            SKIP => pass_explorer.advance(false)?,
            _ => ()
        }

        if pass_explorer.incoming_pass().is_none() {
            break;
        }
    }

    writeln!(term_stdout, "{}", RESTORE_SCREEN)?;
    term_stdout.show_cursor()?;

    Ok(())
}
