use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use rhai::EvalAltResult;
use std::{fs, path::Path};

use super::exec_scripts::RhaiResult;

pub(super) trait RhaiReport {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        msg: S,
    );

    fn report<P: AsRef<Path>>(&self, path: P) {
        self.report_raw(path, 0, "")
    }
}

impl RhaiReport for rhai::Position {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        len: usize,
        msg: S,
    ) {
        let source =
            fs::read_to_string(path.as_ref()).expect("Failed to open file");
        let name = path.as_ref().to_str().unwrap();

        if let (Some(line), Some(position)) = (self.line(), self.position()) {
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
                .with_message("Failed to load plugin")
                .with_label(
                    Label::new((name, err_offset..err_offset + len))
                        .with_message(msg.as_ref().fg(Color::Red)),
                )
                .finish()
                .eprint((name, Source::from(source)))
                .unwrap()
        } else {
            eprintln!("Failed to load plugin {name}");
            eprintln!("  {}", msg.as_ref());
        }
    }
}

impl<T> RhaiReport for RhaiResult<T> {
    fn report_raw<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        _len: usize,
        _msg: S,
    ) {
        if let Err(e) = self {
            match &**e {
                EvalAltResult::ErrorVariableNotFound(variable, pos) => {
                    pos.report_raw(&path, variable.len(), "Undefined variable")
                }
                EvalAltResult::ErrorFunctionNotFound(msg, pos) => {
                    let (fn_name, args) = msg.split_once(' ').unwrap();
                    pos.report_raw(
                        &path,
                        fn_name.len(),
                        format!("{fn_name} {args}"),
                    )
                }
                // for errors that we don't have custom processing, just point
                // to the beginning of the error, and use the error Display as message
                e => e.position().report_raw(&path, 0, format!("{e}")),
            }
        }
    }
}
