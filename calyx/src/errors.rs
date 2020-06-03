//! This file contains the centralized error handling for Futil. Each variant of the
//! `Error` enum represents a different type of error. For some types of errors, you
//! might want to add a `From` impl so that the `?` syntax is more convienent.

// XXX(Sam) Add a proper printer for error types

use crate::frontend::{library_syntax, syntax};
use crate::lang::ast;
use std::iter::repeat;
use std::rc::Rc;

pub enum Error {
    UnknownPass(String, String),
    InvalidFile,
    ParseError(pest_consume::Error<syntax::Rule>),
    LibraryParseError(pest_consume::Error<library_syntax::Rule>),
    WriteError,
    MismatchedPortWidths(ast::Port, u64, ast::Port, u64),
    UndefinedPort(String),
    UndefinedEdge(String, String),
    UndefinedComponent(ast::Id),
    SignatureResolutionFailed(ast::Id),
    DuplicatePort(ast::Id, ast::Portdef),
    MalformedControl(String),
    MalformedStructure(String),
    MissingImplementation(&'static str, ast::Id),
    Impossible(String), // Signal compiler errors that should never occur.
    NotSubcomponent,
    #[allow(unused)]
    Misc(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    input: Rc<String>,
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(span: pest::Span, input: Rc<String>) -> Span {
        Span {
            input,
            start: span.start(),
            end: span.end(),
        }
    }

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
                    repeat("-").take(self.end - self.start - 1).collect();
                let space: String =
                    repeat(" ").take(self.start - pos).collect();
                buf += "\n";
                buf += &format!("{}|{}\n", linum_text, l);
                buf += &format!(
                    "{}|{}^{} {}\n",
                    linum_space, space, mark, err_msg
                );
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
            UnknownPass(pass, known_passes) => {
                write!(
                    f,
                    "Unknown pass: {}. Known passes: {}.",
                    pass,
                    known_passes
                )
            },
            InvalidFile => write!(f, "InvalidFile"),
            ParseError(err) => write!(f, "{}", err),
            LibraryParseError(err) => write!(f, "{}", err),
            WriteError => write!(f, "WriteError"),
            MismatchedPortWidths(port1, w1, port2, w2) => write!(
                f,
                "Mismatched Port Widths: {:?} ({}) != {:?} ({})",
                port1, w1, port2, w2
            ),
            UndefinedPort(port) => write!(f, "Use of undefined port: {}", port),
            UndefinedEdge(src, dest) => write!(f, "Use of undefined edge: {}->{}", src, dest),
            UndefinedComponent(id) => {
                write!(f, "Use of undefined component {}", id.to_string())
            }
            SignatureResolutionFailed(id) => {
                let msg = "Undefined identifier";
                write!(f, "{}", id.fmt_err(msg))
            }
            DuplicatePort(comp, portdef) => {
                write!(f, "Attempted to add `{:?}` to component `{}`", portdef, comp.to_string())
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

impl From<std::io::Error> for Error {
    fn from(_err: std::io::Error) -> Self {
        Error::InvalidFile
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(_err: std::str::Utf8Error) -> Self {
        Error::InvalidFile
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_err: std::fmt::Error) -> Self {
        Error::WriteError
    }
}

impl From<pest_consume::Error<syntax::Rule>> for Error {
    fn from(e: pest_consume::Error<syntax::Rule>) -> Self {
        Error::ParseError(e)
    }
}

impl From<pest_consume::Error<library_syntax::Rule>> for Error {
    fn from(e: pest_consume::Error<library_syntax::Rule>) -> Self {
        Error::LibraryParseError(e)
    }
}
