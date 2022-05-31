//! Errors generated by the compiler.

use itertools::Itertools;

use crate::frontend::parser;
use crate::ir;
use std::cmp;
use std::rc::Rc;

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

    /// Returns the
    /// 1. lines associated with this span
    /// 2. start position of the first line in span
    /// 3. line number of the span
    fn get_lines(&self) -> (Vec<&str>, usize, usize) {
        let lines = self.input.split('\n').collect_vec();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        let mut collect_lines = false;
        let mut buf = Vec::new();

        let mut out_line: usize = 0;
        let mut out_idx: usize = 0;
        for l in lines {
            let next_pos = pos + l.len();
            if self.start >= pos && self.start <= next_pos {
                out_line = linum;
                out_idx = pos;
                collect_lines = true;
            }
            if collect_lines && self.end >= pos {
                buf.push(l)
            }
            if self.end <= next_pos {
                break;
            }
            pos = next_pos + 1;
            linum += 1;
        }
        (buf, out_idx, out_line)
    }

    /// Format this Span with a the error message `err_msg`
    pub fn format(&self, err_msg: &str) -> String {
        let (lines, pos, linum) = self.get_lines();
        let mut buf = self.file.to_string();

        let l = lines[0];
        let linum_text = format!("{} ", linum);
        let linum_space: String = " ".repeat(linum_text.len());
        let mark: String = "^".repeat(cmp::min(
            self.end - self.start,
            l.len() - (self.start - pos),
        ));
        let space: String = " ".repeat(self.start - pos);
        buf += "\n";
        buf += &format!("{}|{}\n", linum_text, l);
        buf += &format!("{}|{}{} {}", linum_space, space, mark, err_msg);
        buf
    }

    /// Visualizes the span without any message or mkaring
    pub fn show(&self) -> String {
        let (lines, _, linum) = self.get_lines();
        let l = lines[0];
        let linum_text = format!("{} ", linum);
        format!("{}|{}\n", linum_text, l)
    }
}

/// An IR node that may contain position information.
pub trait WithPos {
    /// Copy the span associated with this node.
    fn copy_span(&self) -> Option<Span>;
}

pub struct Error {
    kind: ErrorKind,
    pos: Option<Span>,
    post_msg: Option<String>,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.pos {
            None => write!(f, "{}", self.kind)?,
            Some(pos) => write!(f, "{}", pos.format(&self.kind.to_string()))?,
        }
        if let Some(post) = &self.post_msg {
            write!(f, "\n{}", post)?;
        }
        Ok(())
    }
}

impl Error {
    pub fn with_pos<T: WithPos>(mut self, pos: &T) -> Self {
        self.pos = pos.copy_span();
        self
    }

    pub fn with_post_msg(mut self, msg: Option<String>) -> Self {
        self.post_msg = msg;
        self
    }

    pub fn parse_error(err: pest_consume::Error<parser::Rule>) -> Self {
        Self {
            kind: ErrorKind::ParseError(err),
            pos: None,
            post_msg: None,
        }
    }

    pub fn unallowed_type(msg: String) -> Self {
        Self { 
            kind: ErrorKind::UnallowedType(msg), 
            pos: None, 
            post_msg: None,
        }
    }
    pub fn reserved_name(name: ir::Id) -> Self {
        Self {
            kind: ErrorKind::ReservedName(name),
            pos: None,
            post_msg: None,
        }
    }
    pub fn malformed_control(msg: String) -> Self {
        Self {
            kind: ErrorKind::MalformedControl(msg),
            pos: None,
            post_msg: None,
        }
    }
    pub fn malformed_structure<S: ToString>(msg: S) -> Self {
        Self {
            kind: ErrorKind::MalformedStructure(msg.to_string()),
            pos: None,
            post_msg: None,
        }
    }
    pub fn pass_assumption(pass: String, msg: String) -> Self {
        Self {
            kind: ErrorKind::PassAssumption(pass, msg),
            pos: None,
            post_msg: None,
        }
    }
    pub fn undefined(name: ir::Id, typ: String) -> Self {
        let pos = name.copy_span();
        Self {
            kind: ErrorKind::Undefined(name, typ),
            pos,
            post_msg: None,
        }
    }
    pub fn already_bound(name: ir::Id, typ: String) -> Self {
        let pos = name.copy_span();
        Self {
            kind: ErrorKind::AlreadyBound(name, typ),
            pos,
            post_msg: None,
        }
    }
    pub fn unused<S: ToString>(group: ir::Id, typ: S) -> Self {
        Self {
            kind: ErrorKind::Unused(group, typ.to_string()),
            pos: None,
            post_msg: None,
        }
    }
    pub fn papercut(msg: String) -> Self {
        Self {
            kind: ErrorKind::Papercut(msg),
            pos: None,
            post_msg: None,
        }
    }
    pub fn misc(msg: String) -> Self {
        Self {
            kind: ErrorKind::Misc(msg),
            pos: None,
            post_msg: None,
        }
    }
    pub fn invalid_file(msg: String) -> Self {
        Self {
            kind: ErrorKind::InvalidFile(msg),
            pos: None,
            post_msg: None,
        }
    }
    pub fn write_error(msg: String) -> Self {
        Self {
            kind: ErrorKind::WriteError(msg),
            pos: None,
            post_msg: None,
        }
    }
}

/// Standard error type for Calyx errors.
pub enum ErrorKind {
    /// Error while parsing a Calyx program.
    ParseError(pest_consume::Error<parser::Rule>),
    /// Using a reserved keyword as a program identifier.
    ReservedName(ir::Id),

    /// Prototype not allowed for external cells.
    UnallowedType(String),

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
    Unused(ir::Id, String),

    /// Papercut error: signals a commonly made mistake in Calyx program.
    Papercut(String),

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

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ErrorKind::*;
        match self {
            Papercut(msg) => {
                write!(f, "[Papercut] {}", msg)
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
            Unused(name, typ) => {
                write!(f, "Unused {typ} `{name}'")
            }
            AlreadyBound(name, bound_by) => {
                write!(f, "Name `{name}' already bound by {bound_by}")
            }
            ReservedName(name) => {
                write!(f, "Use of reserved keyword: {name}")
            }
            Undefined(name, typ) => {
                write!(f, "Undefined {typ} name: {name}")
            }
            ParseError(err) => write!(f, "Calyx Parser: {err}"),
            MalformedControl(msg) => write!(f, "Malformed Control: {msg}"),
            PassAssumption(pass, msg) => {
                write!(f, "Pass `{pass}` assumption violated: {msg}")
            }
            MalformedStructure(msg) => {
                write!(f, "Malformed Structure: {msg}")
            }
            InvalidFile(msg) | WriteError(msg) | Misc(msg) => {
                write!(f, "{msg}")
            }
            UnallowedType(msg) => {write!(f, "Type Not Allowed: {msg}")}
        }
    }
}

// Conversions from other error types to our error type so that
// we can use `?` in all the places.
impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::invalid_file(err.to_string())
    }
}

impl From<pest_consume::Error<parser::Rule>> for Error {
    fn from(e: pest_consume::Error<parser::Rule>) -> Self {
        Error::parse_error(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::write_error(format!("IO Error: {}", e))
    }
}
