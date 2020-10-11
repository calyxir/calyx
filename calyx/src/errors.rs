//! This file contains the centralized error handling for FuTIL. Each variant of the
//! `Error` enum represents a different type of error. For some types of errors, you
//! might want to add a `From` impl so that the `?` syntax is more convienent.

use crate::frontend::{library_parser, parser};
use crate::lang::ast;
use petgraph::stable_graph::NodeIndex;
use std::iter::repeat;
use std::rc::Rc;

/// Standard error type for FuTIL errors.
#[allow(clippy::large_enum_variant)]
pub enum Error {
    /// Error while parsing a FuTIL program.
    ParseError(pest_consume::Error<parser::Rule>),
    /// Error while parsing a FuTIL library.
    LibraryParseError(pest_consume::Error<library_parser::Rule>),
    /// Using a reserved keyword as a program identifier.
    ReservedName(ast::Id),

    /// The given string does not correspond to any known pass.
    UnknownPass(String, String),
    /// The input file is invalid (does not exist).
    InvalidFile(String),
    /// Failed to write the output
    WriteError,

    /// The control program is malformed.
    MalformedControl(String),

    /// The connections are malformed.
    MalformedStructure(String),
    /// The port widths don't match up on an edge.
    MismatchedPortWidths(ast::Port, u64, ast::Port, u64),
    /// Port not found on the given component.
    UndefinedPort(ast::Id, String),
    /// The component has not been defined.
    UndefinedComponent(ast::Id),
    /// The group has not been defined
    UndefinedGroup(ast::Id),
    /// The group was not used in the program.
    UnusedGroup(ast::Id),

    /// The name has already been bound.
    AlreadyBound(ast::Id, String),
    /// The group has already been bound.
    DuplicateGroup(ast::Id),
    /// The port has already been defined.
    DuplicatePort(ast::Id, ast::Portdef),

    /// No value provided for a primitive parameter.
    SignatureResolutionFailed(ast::Id, ast::Id),

    /// An implementation is missing.
    MissingImplementation(&'static str, ast::Id),

    /// Papercut error: signals a commonly made mistake in FuTIL program.
    Papercut(String, ast::Id),

    /// Internal compiler error that should never occur.
    Impossible(String), // Signal compiler errors that should never occur.
    NotSubcomponent,

    /// A miscellaneous error. Should be replaced with a more precise error.
    #[allow(unused)]
    Misc(String),
}

pub type FutilResult<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    // we use Rc<String> here so that we don't have tostore the entire
    // input program for each identifier and Rc<String> has nicer lifetimes than &str.
    input: Rc<String>,
    start: usize,
    end: usize,
}

impl Span {
    /// Create a new `Error::Span` from a `pest::Span` and
    /// the input string.
    pub fn new(span: pest::Span, input: Rc<String>) -> Span {
        Span {
            input,
            start: span.start(),
            end: span.end(),
        }
    }

    /// Format this Span with a the error message `err_msg`
    pub fn format(&self, err_msg: &str) -> String {
        let lines = self.input.split('\n');
        let mut buf: String = String::new();
        let mut pos: usize = 0;
        let mut linum: usize = 1;
        for l in lines {
            let new_pos = pos + l.len() + 1;
            if self.start > pos && self.end < pos + (l.len()) {
                let linum_text = format!("{} ", linum);
                let linum_space: String =
                    repeat(" ").take(linum_text.len()).collect();
                let mark: String =
                    repeat("^").take(self.end - self.start).collect();
                let space: String =
                    repeat(" ").take(self.start - pos).collect();
                buf += "\n";
                buf += &format!("{}|{}\n", linum_text, l);
                buf +=
                    &format!("{}|{}{} {}", linum_space, space, mark, err_msg);
                break;
            }
            pos = new_pos;
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
            UnusedGroup(name) => {
                write!(
                    f,
                    "{}",
                    name.fmt_err("Group not used in control")
                )
            }
            AlreadyBound(name, bound_by) => {
                let msg = format!("Name already bound by {}", bound_by.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            ReservedName(name) => {
                let msg = format!("Use of reserved keyword: {}", name.to_string());
                write!(f, "{}", name.fmt_err(&msg))
            }
            UndefinedGroup(name) => {
                let msg = format!("Use of undefined group: {}", name.to_string());
                write!(
                    f,
                    "{}",
                    name.fmt_err(&msg)
                )
            }
            UnknownPass(pass, known_passes) => {
                write!(
                    f,
                    "Unknown pass: {}. Known passes: {}.",
                    pass,
                    known_passes
                )
            },
            InvalidFile(err) => write!(f, "InvalidFile: {}", err),
            ParseError(err) => write!(f, "FuTIL Parser: {}", err),
            LibraryParseError(err) => write!(f, "FuTIL Library Parser: {}", err),
            WriteError => write!(f, "WriteError"),
            MismatchedPortWidths(port1, w1, port2, w2) => {
                let msg1 = format!("This port has width: {}", w1);
                let msg2 = format!("This port has width: {}", w2);
                write!(f, "{}\nwhich doesn't match the width of '{}':{}",
                       port1.port_name().fmt_err(&msg1),
                       port2.port_name().to_string(),
                       port2.port_name().fmt_err(&msg2))
            }
            UndefinedPort(port, port_kind) => {
                let msg = format!("Use of undefined {} port: {}", port_kind, port.to_string());
                write!(f, "{}", port.fmt_err(&msg))
            }
            UndefinedComponent(id) => {
                let msg = format!("Use of undefined component: {}", id.to_string());
                write!(f, "{}", id.fmt_err(&msg))
            }
            SignatureResolutionFailed(id, param_name) => {
                let msg = format!("No value passed in for parameter: {}", param_name.to_string());
                write!(f, "{}\nwhich is used here:{}", id.fmt_err(&msg), param_name.fmt_err(""))
            }
            DuplicateGroup(group) => {
                write!(f, "Attempted to duplicate group `{}`", group.to_string())
            }
            DuplicatePort(comp, portdef) => {
                write!(f, "Attempted to add duplicate port `{}` to component `{}`", portdef.to_string(), comp.to_string())
            }
            MalformedControl(msg) => write!(f, "Malformed Control: {}", msg),
            MalformedStructure(msg) => write!(f, "Malformed Structure: {}", msg),
            NotSubcomponent => write!(f, "Not a subcomponent"),
            Misc(msg) => write!(f, "{}", msg),
            Impossible(msg) => write!(f, "Impossible: {}\nThis error should never occur. Report report this as a bug.", msg),
            MissingImplementation(name, id) => write!(f, "Mising {} implementation for `{}`", name, id.to_string())
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

impl From<std::fmt::Error> for Error {
    fn from(_err: std::fmt::Error) -> Self {
        Error::WriteError
    }
}

impl From<pest_consume::Error<parser::Rule>> for Error {
    fn from(e: pest_consume::Error<parser::Rule>) -> Self {
        Error::ParseError(e)
    }
}

impl From<pest_consume::Error<library_parser::Rule>> for Error {
    fn from(e: pest_consume::Error<library_parser::Rule>) -> Self {
        Error::LibraryParseError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(_e: std::io::Error) -> Self {
        Error::WriteError
    }
}

// Utility traits

/// A generalized 'unwrapping' trait that extracts data from
/// a container that can possible be an error and automatically
/// generates the correct `Error` variant with the `ast::Id`.
/// For example, `Extract<NodeIndex, NodeIndex>` can be implemented for
/// `Option<NodeIndex>` to provide convienent error reporting for
/// undefined components / groups.
pub trait Extract<T, R> {
    /// Unpacks `T` into `FutilResult<R>` using `id: ast::Id`
    /// for error reporting with locations.
    fn extract(&self, id: &ast::Id) -> FutilResult<R>;
}

impl Extract<NodeIndex, NodeIndex> for Option<NodeIndex> {
    fn extract(&self, id: &ast::Id) -> FutilResult<NodeIndex> {
        match self {
            Some(t) => Ok(*t),
            None => Err(Error::UndefinedComponent(id.clone())),
        }
    }
}
