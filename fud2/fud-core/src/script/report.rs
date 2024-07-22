use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use rhai::EvalAltResult;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{error::RhaiSystemError, exec_scripts::RhaiResult};

/// A small hack to improve error messages. These are known function names
/// so that we can say that arguments are incorrect when a user calls
/// one of these functions.
const KNOWN_FNS: [&str; 4] = ["state", "op", "get_state", "get_setup"];

pub(super) trait RhaiReport {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        header: Option<String>,
        msg: S,
    );

    fn report<P: AsRef<Path>>(&self, path: P) {
        self.report_raw(path, 0, None, "")
    }
}

impl RhaiReport for rhai::Position {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        header: Option<String>,
        msg: S,
    ) {
        let source = fs::read_to_string(path.as_ref());
        let name = path.as_ref().to_str().unwrap();

        if let (Some(line), Ok(source)) = (self.line(), source) {
            let position = self.position().unwrap_or(1);
            // translate a line offset into a char offset
            let line_offset = source
                .lines()
                // take all the lines up to pos.line()
                .take(line - 1)
                // add one to all the line lengths because `\n` chars are rmeoved with `.lines()`
                .map(|line| line.len() + 1)
                .sum::<usize>();

            // add the column offset to get the beginning of the error
            // we subtract 1, because the positions are 1 indexed
            let err_offset = line_offset + (position - 1);

            Report::build(ReportKind::Error, name, err_offset)
                .with_message(
                    header.unwrap_or("Failed to load plugin".to_string()),
                )
                .with_label(
                    Label::new((name, err_offset..err_offset + len))
                        .with_message(msg.as_ref().fg(Color::Red)),
                )
                .finish()
                .eprint((name, Source::from(source)))
                .unwrap()
        } else {
            eprintln!("Failed to load plugin: {name}");
            let pos_str = if self.is_none() {
                "".to_string()
            } else {
                format!(" @ {self}")
            };
            eprintln!("  {}{pos_str}", msg.as_ref());
        }
    }
}

impl RhaiReport for EvalAltResult {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        _len: usize,
        header: Option<String>,
        _msg: S,
    ) {
        match &self {
            EvalAltResult::ErrorVariableNotFound(variable, pos) => pos
                .report_raw(
                    &path,
                    variable.len(),
                    header,
                    "Undefined variable",
                ),
            EvalAltResult::ErrorFunctionNotFound(msg, pos) => {
                let (fn_name, args) = msg.split_once(' ').unwrap_or((msg, ""));
                let msg = if KNOWN_FNS.contains(&fn_name) {
                    format!("Invalid arguments. Expected {args}")
                } else {
                    format!("Unknown function: {fn_name} {args}")
                };
                pos.report_raw(&path, fn_name.len(), header, msg)
            }
            EvalAltResult::ErrorSystem(msg, err)
                if err.is::<RhaiSystemError>() =>
            {
                let e = err.downcast_ref::<RhaiSystemError>().unwrap();
                let msg = if msg.is_empty() {
                    format!("{err}")
                } else {
                    format!("{msg}: {err}")
                };
                e.position.report_raw(&path, 0, header, msg)
            }
            EvalAltResult::ErrorInModule(submod_path, err, _)
                if path.as_ref().to_str() == Some(submod_path) =>
            {
                err.report(submod_path)
            }
            EvalAltResult::ErrorInModule(submod_path, err, _) => err
                .report_raw(
                    submod_path,
                    0,
                    Some(format!(
                        "Error in submodule {:?} while loading {:?}",
                        PathBuf::from(submod_path).file_stem().unwrap(),
                        path.as_ref().file_stem().unwrap()
                    )),
                    "",
                ),
            // for errors that we don't have custom processing, just point
            // to the beginning of the error, and use the error Display as message
            e => e.position().report_raw(&path, 0, header, format!("{e}")),
        }
    }
}

impl<T> RhaiReport for RhaiResult<T> {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        header: Option<String>,
        msg: S,
    ) {
        if let Err(e) = self {
            (**e).report_raw(path, len, header, msg);
        }
    }
}
