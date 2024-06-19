//! Errors generated by the compiler.
use crate::{GPosIdx, Id, WithPos};

/// Convience wrapper to represent success or meaningul compiler error.
pub type CalyxResult<T> = std::result::Result<T, Error>;

/// Errors generated by the compiler
#[derive(Clone)]
pub struct Error {
    kind: Box<ErrorKind>,
    pos: GPosIdx,
    post_msg: Option<String>,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pos == GPosIdx::UNKNOWN {
            write!(f, "{}", self.kind)?
        } else {
            write!(f, "{}", self.pos.format(self.kind.to_string()))?
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

    pub fn reserved_name(name: Id) -> Self {
        Self {
            kind: Box::new(ErrorKind::ReservedName(name)),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn malformed_control<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::MalformedControl(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn malformed_structure<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::MalformedStructure(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn pass_assumption<S: ToString, M: ToString>(pass: S, msg: M) -> Self {
        Self {
            kind: Box::new(ErrorKind::PassAssumption(
                pass.to_string(),
                msg.to_string(),
            )),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn undefined<S: ToString>(name: Id, typ: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::Undefined(name, typ.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn already_bound<S: ToString>(name: Id, typ: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::AlreadyBound(name, typ.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn unused<S: ToString>(group: Id, typ: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::Unused(group, typ.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn papercut<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::Papercut(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn misc<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::Misc(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn parse_error<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::Parse),
            pos: GPosIdx::UNKNOWN,
            post_msg: Some(msg.to_string()),
        }
    }
    pub fn invalid_file<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::InvalidFile(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn write_error<S: ToString>(msg: S) -> Self {
        Self {
            kind: Box::new(ErrorKind::WriteError(msg.to_string())),
            pos: GPosIdx::UNKNOWN,
            post_msg: None,
        }
    }
    pub fn location(&self) -> (&str, usize, usize) {
        self.pos.get_location()
    }
    pub fn message(&self) -> String {
        self.kind.to_string()
    }
}

/// Standard error type for Calyx errors.
#[derive(Clone)]
enum ErrorKind {
    /// Using a reserved keyword as a program identifier.
    ReservedName(Id),

    /// The control program is malformed.
    MalformedControl(String),
    /// The connections are malformed.
    MalformedStructure(String),

    /// Requirement of a pass was not satisfied
    PassAssumption(String, String),

    /// The name has not been bound
    Undefined(Id, String),
    /// The name has already been bound.
    AlreadyBound(Id, String),

    /// The group was not used in the program.
    Unused(Id, String),

    /// Papercut error: signals a commonly made mistake in Calyx program.
    Papercut(String),

    // =========== Frontend Errors ===============
    /// Parse error
    Parse,
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
            MalformedControl(msg) => write!(f, "Malformed Control: {msg}"),
            PassAssumption(pass, msg) => {
                write!(f, "Pass `{pass}` assumption violated: {msg}")
            }
            MalformedStructure(msg) => {
                write!(f, "Malformed Structure: {msg}")
            }
            Parse => {
                write!(f, "Parse error")
            }
            InvalidFile(msg) | WriteError(msg) | Misc(msg) => {
                write!(f, "{msg}")
            }
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

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::write_error(format!("IO Error: {}", e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::write_error(format!("serde_json Error: {}", e))
    }
}
