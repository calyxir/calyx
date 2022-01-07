//! Errors generated by the compiler.

use crate::frontend::parser;
use crate::ir;
use std::rc::Rc;

/// Standard error type for Calyx errors.
pub enum Error {
    /// Error while parsing a Calyx program.
    ParseError(pest_consume::Error<parser::Rule>),
    /// Using a reserved keyword as a program identifier.
    ReservedName(ir::Id),

    /// The control program is malformed.
    MalformedControl(String),
    /// The connections are malformed.
    MalformedStructure(String),

    /// Requirement of a pass was not satisfied
    PassAssumption(String, String),

    /// The name has not been bound
    Undefined(ir::Id, String),
    /// The name has already been bound.
    AlreadyBound(ir::Id, String),

    /// The group was not used in the program.
    UnusedGroup(ir::Id),

    /// No value provided for a primitive parameter.
    ParamBindingMissing(ir::Id, ir::Id),
    /// Invalid parameter binding provided
    InvalidParamBinding(ir::Id, usize, usize),

    /// Papercut error: signals a commonly made mistake in Calyx program.
    Papercut(String, ir::Id),

    /// Group "static" latency annotation differed from inferred latency.
    ImpossibleLatencyAnnotation(String, u64, u64),

    // =========== Frontend Errors ===============
    /// Miscellaneous error message
    Misc(String),
    /// The input file is invalid (does not exist).
    InvalidFile(String),
    /// Failed to write the output
    WriteError(String),
}

/// Convience wrapper to represent success or meaningul compiler error.
pub type CalyxResult<T> = std::result::Result<T, Error>;

/// A span of the input program.
/// Used for reporting location-based errors.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Reference to input program source.
    input: Rc<str>,
    /// Name of the input file
    file: Rc<str>,
    /// The start of the span.
    start: usize,
    /// The end of the span.
    end: usize,
}

impl Span {
    /// Create a new `Error::Span` from a `pest::Span` and
    /// the input string.
    pub fn new(span: pest::Span, file: Rc<str>, input: Rc<str>) -> Span {
        Span {
            input,
            file,
            start: span.start(),
            end: span.end(),
        }
    }

    /// Format this Span with a the error message `err_msg`
    pub fn format(&self, err_msg: &str) -> String {
        let lines = self.input.split('\n');
        let mut buf = self.file.to_string();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        for l in lines {
            let next_pos = pos + l.len();
            if self.start > pos && self.end <= next_pos {
                let linum_text = format!("{} ", linum);
                let linum_space: String = " ".repeat(linum_text.len());
                let mark: String = "^".repeat(self.end - self.start);
                let space: String = " ".repeat(self.start - pos);
                buf += "\n";
                buf += &format!("{}|{}\n", linum_text, l);
                buf +=
                    &format!("{}|{}{} {}", linum_space, space, mark, err_msg);
                break;
            }
            pos = next_pos + 1;
            linum += 1;
        }
        buf
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            Papercut(msg, id) => {
                write!(f, "{}", id.fmt_err(&("[Papercut] ".to_string() + msg)))
            }
            ImpossibleLatencyAnnotation(grp_name, ann_val, inferred_val) => {
                let msg1 = format!("Annotated latency: {}", ann_val);
                let msg2 = format!("Inferred latency: {}", inferred_val);
                write!(
                    f,
                    "Impossible \"static\" latency annotation for group {}.\n{}\n{}",
                    grp_name,
                    msg1,
                    msg2
                )
            }
            UnusedGroup(name) => {
                write!(f, "{}", name.fmt_err("Group not used in control"))
            }
            AlreadyBound(name, bound_by) => {
                let msg =
                    format!("Name already bound by {}", bound_by.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            ReservedName(name) => {
                let msg =
                    format!("Use of reserved keyword: {}", name.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            Undefined(name, typ) => {
                let msg =
                    format!("Undefined {} name: {}", typ, name.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            InvalidFile(err) => write!(f, "{}", err),
            WriteError(msg) => write!(f, "{}", msg),
            ParseError(err) => write!(f, "Calyx Parser: {}", err),
            ParamBindingMissing(id, param_name) => {
                let msg =
                    format!("Failed to resolve: {}", param_name.to_string());
                write!(
                    f,
                    "{}\nwhich is used here:{}",
                    id.fmt_err(&msg),
                    param_name.fmt_err("")
                )
            }
            InvalidParamBinding(prim, param_len, bind_len) => {
                let msg = format!(
                    "Invalid parameter binding for primitive `{}`. Requires {} parameters but provided with {}.",
                    prim,
                    param_len,
                    bind_len
                );
                write!(f, "{}", msg)
            }
            MalformedControl(msg) => write!(f, "Malformed Control: {}", msg),
            PassAssumption(pass, msg) => {
                write!(f, "Pass `{}` assumption violated: {}", pass, msg)
            }
            MalformedStructure(msg) => {
                write!(f, "Malformed Structure: {}", msg)
            }
            Misc(msg) => write!(f, "{}", msg),
        }
    }
}

// Conversions from other error types to our error type so that
// we can use `?` in all the places.

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::InvalidFile(err.to_string())
    }
}

impl From<pest_consume::Error<parser::Rule>> for Error {
    fn from(e: pest_consume::Error<parser::Rule>) -> Self {
        Error::ParseError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::WriteError(format!("IO Error: {}", e))
    }
}
