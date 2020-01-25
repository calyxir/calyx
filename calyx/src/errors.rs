use crate::lang::ast;

pub enum Error {
    InvalidFile,
    ParseError(String),
    WriteError,
    MismatchedPortWidths(ast::Port, i64, ast::Port, i64),
    UndefinedPort(String),
    UndefinedComponent(ast::Id),
    SignatureResolutionFailed(ast::Id),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
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
