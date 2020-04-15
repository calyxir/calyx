//! This file contains the centralized error handling for Futil. Each variant of the
//! `Error` enum represents a different type of error. For some types of errors, you
//! might want to add a `From` impl so that the `?` syntax is more convienent.

// XXX(Sam) Add a proper printer for error types

use crate::lang::ast;

pub enum Error {
    UnknownPass(String, String),
    InvalidFile,
    ParseError(String),
    WriteError,
    MismatchedPortWidths(ast::Port, u64, ast::Port, u64),
    UndefinedPort(String),
    UndefinedComponent(ast::Id),
    SignatureResolutionFailed(ast::Id),
    MalformedControl,   // XXX(sam) add more info to this
    Impossible(String), // Signal compiler errors that should never occur.
    #[allow(unused)]
    Misc(String),
    // XXX(ken) interpreter errors
    StructureHasCycle,
    InvalidConstant(String, i64, u64) //the value of constant exceeds p-width
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
            ParseError(msg) => write!(f, "{}", msg),
            WriteError => write!(f, "WriteError"),
            MismatchedPortWidths(port1, w1, port2, w2) => write!(
                f,
                "Mismatched Port Widths: {:?} ({}) != {:?} ({})",
                port1, w1, port2, w2
            ),
            UndefinedPort(port) => write!(f, "Use of undefined port: {}", port),
            UndefinedComponent(id) => {
                write!(f, "Use of undefined component {:?}", id)
            }
            SignatureResolutionFailed(id) => {
                write!(f, "Failed to resolve portdef: {:?}", id)
            }
            MalformedControl => write!(f, "Malformed Control. Backend expected Control to be in a different form."),
            Misc(msg) => write!(f, "{}", msg),
            Impossible(msg) => write!(f, "Impossible: {}\nThis error should never occur. Report report this as a bug.", msg),
            StructureHasCycle => write!(f, "Structure has a cycle that does not contain a sequential state component. This is undefined behavior."),
            InvalidConstant(port, width, value) => write!(f, "")
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

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::ParseError(s)
    }
}
