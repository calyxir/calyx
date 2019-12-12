#[derive(Debug)]
pub enum Error {
    InvalidFile,
    ParseError,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::InvalidFile
    }
}

impl From<Box<sexp::Error>> for Error {
    fn from(err: Box<sexp::Error>) -> Self {
        Error::ParseError
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::InvalidFile
    }
}
