use calyx_pass::{
    cli::ParseArgs,
    pass_explorer::{PassApplicationStatus, PassExplorer},
    tui::ScrollbackBuffer,
    util,
};
use colored::Colorize;
use console::{Key, Term};
use std::{
    cmp::{max, min},
    io::Write,
    path::PathBuf,
};
use tempdir::TempDir;

/// Saves the terminal buffer.
///
/// Source: https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797
const SAVE_SCREEN: &str = "\x1b[?47h";

/// Restores the terminal buffer. See source at [`SAVE_SCREEN`].
const RESTORE_SCREEN: &str = "\x1b[?47l";

/// Switches to the alternative buffer. See source at [`SAVE_SCREEN`].
const SWITCH_ALTERNATIVE_BUFFER: &str = "\x1b[?1049h";

/// Restores the main buffer. See source at [`SAVE_SCREEN`].
const SWITCH_MAIN_BUFFER: &str = "\x1b[?1049l";

/// Saves the current cursor position. See source at [`SAVE_SCREEN`].
const SAVE_CURSOR: &str = "\x1b8";

/// Restores the saved cursor position. See source at [`SAVE_SCREEN`].
const RESTORE_CURSOR: &str = "\x1b9";

#[allow(clippy::write_literal)]
#[allow(clippy::needless_range_loop)]
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

    // use . for tmpdir for debugging, eventually just use TempDir::new
    let temp_dir = TempDir::new_in(".", ".calyx-pass")?;
    let mut pass_explorer = PassExplorer::new(
        temp_dir,
        args.calyx_exec,
        args.breakpoint,
        args.pass_alias,
        PathBuf::from(args.input_file),
    )?;

    /// Quit the program.
    const QUIT: char = 'q';

    /// See [`PassExplorer::accept`].
    const ACCEPT: char = 'a';

    /// See [`PassExplorer::skip`].
    const SKIP: char = 's';

    /// See [`PassExplorer::undo`].
    const UNDO: char = 'u';

    /// Scroll forward [`JUMP`] lines.
    const JUMP_FWD: char = 'f';

    /// Scroll backward [`JUMP`] lines.
    const JUMP_BCK: char = 'b';

    /// See [`JUMP_FWD`] and [`JUMP_BCK`].
    const JUMP: usize = 4;

    let mut term_stdout = Term::stdout();
    writeln!(term_stdout, "{}", SAVE_SCREEN)?;
    writeln!(term_stdout, "{}", SAVE_CURSOR)?;
    writeln!(term_stdout, "{}", SWITCH_ALTERNATIVE_BUFFER)?;
    term_stdout.hide_cursor()?;

    let mut scrollback_buffer = ScrollbackBuffer::new(&term_stdout);
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            writeln!(
                scrollback_buffer,
                "{}",
                "Calyx Pass Explorer".underline()
            )?;
            writeln!(
                scrollback_buffer,
                "Usage:\n  1. Analysis: {} {}, {} {}, {} {}, {} {}\n  2. Movement: {} {}, {} {}, up/down arrows, scroll",
                ACCEPT.to_string().bright_green(),
                "accept".green(),
                SKIP,
                "skip",
                QUIT.to_string().bright_red(),
                "quit".red(),
                UNDO.to_string().bright_cyan(),
                "undo".cyan(),
                JUMP_FWD.to_string().bright_magenta(),
                "forward".magenta(),
                JUMP_BCK.to_string().bright_magenta(),
                "back".magenta(),
            )?;

            let current_pass_application =
                pass_explorer.current_pass_application();
            if let Some(incoming_pos) =
                current_pass_application.iter().position(|(_, status)| {
                    *status == PassApplicationStatus::Incoming
                })
            {
                write!(scrollback_buffer, "Passes: ")?;
                let start_index = max(0, (incoming_pos as isize) - 3) as usize;
                if start_index > 0 {
                    write!(scrollback_buffer, "[{} more] ... ", start_index)?;
                }

                let length =
                    min(5, current_pass_application.len() - start_index);
                for i in start_index..start_index + length {
                    if i > start_index {
                        write!(scrollback_buffer, ", ")?;
                    }
                    let (name, status) = &current_pass_application[i];
                    let colored_name = match status {
                        PassApplicationStatus::Applied => name.green(),
                        PassApplicationStatus::Skipped => name.dimmed(),
                        PassApplicationStatus::Incoming => {
                            format!("[INCOMING] {}", name).yellow().bold()
                        }
                        PassApplicationStatus::Future => name.purple(),
                    };
                    write!(scrollback_buffer, "{}", colored_name)?;
                }

                let remaining_count =
                    current_pass_application.len() - start_index - length;
                if remaining_count > 0 {
                    write!(
                        scrollback_buffer,
                        " ... [{} more]",
                        remaining_count
                    )?;
                }

                writeln!(scrollback_buffer)?;
            }

            if let Some(review) =
                pass_explorer.review(args.component.clone())?
            {
                let rows = term_stdout.size().1;
                writeln!(
                    scrollback_buffer,
                    "{}",
                    "─".repeat(rows as usize).dimmed()
                )?;
                write!(scrollback_buffer, "{}", review)?;
            }

            needs_redraw = false;
        }

        scrollback_buffer.display()?;

        match term_stdout.read_key()? {
            Key::Char(c) => match c {
                QUIT => break,
                ACCEPT => {
                    pass_explorer.accept()?;
                    scrollback_buffer.clear();
                    needs_redraw = true;
                }
                SKIP => {
                    pass_explorer.skip()?;
                    scrollback_buffer.clear();
                    needs_redraw = true;
                }
                UNDO => {
                    pass_explorer.undo()?;
                    scrollback_buffer.clear();
                    needs_redraw = true;
                }
                JUMP_FWD => {
                    for _ in 0..JUMP {
                        scrollback_buffer.scroll_down()
                    }
                }
                JUMP_BCK => {
                    for _ in 0..JUMP {
                        scrollback_buffer.scroll_up()
                    }
                }
                _ => (),
            },
            Key::ArrowUp => scrollback_buffer.scroll_up(),
            Key::ArrowDown => scrollback_buffer.scroll_down(),
            _ => (),
        }

        if pass_explorer.incoming_pass().is_none() {
            break;
        }
    }

    writeln!(term_stdout, "{}", RESTORE_SCREEN)?;
    writeln!(term_stdout, "{}", RESTORE_CURSOR)?;
    writeln!(term_stdout, "{}", SWITCH_MAIN_BUFFER)?;
    term_stdout.show_cursor()?;
    term_stdout.move_cursor_down(1)?;

    Ok(())
}
