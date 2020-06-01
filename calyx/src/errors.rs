//! This file contains the centralized error handling for Futil. Each variant of the
//! `Error` enum represents a different type of error. For some types of errors, you
//! might want to add a `From` impl so that the `?` syntax is more convienent.

// XXX(Sam) Add a proper printer for error types

use crate::frontend::syntax::Rule;
use crate::lang::ast;

pub enum Error {
    UnknownPass(String, String),
    InvalidFile,
    ParseError(pest_consume::Error<Rule>),
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
            ParseError(msg) => write!(f, "{}", msg),
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
                write!(f, "Failed to resolve portdef: {}", id.to_string())
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

impl From<pest_consume::Error<Rule>> for Error {
    fn from(e: pest_consume::Error<Rule>) -> Self {
        Error::ParseError(e)
    }
}
