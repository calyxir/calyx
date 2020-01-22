use crate::lang::ast;

pub enum Error {
    InvalidFile,
    ParseError(String),
    WriteError,
    UndefinedComponent(ast::Id),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            InvalidFile => write!(f, "InvalidFile"),
            ParseError(msg) => write!(f, "{}", msg),
            WriteError => write!(f, "WriteError"),
            UndefinedComponent(id) => {
                write!(f, "Use of undefined component {:?}", id)
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(_err: std::io::Error) -> Self {
        Error::InvalidFile
    }
}

// impl From<Box<sexp::Error>> for Error {
//     fn from(_err: Box<sexp::Error>) -> Self {
//         Error::ParseError
//     }
// }

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
