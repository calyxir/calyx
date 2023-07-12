use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyAdapterError {
    #[error("Unhandled command")]
    UnhandledCommandError,
    // Add more error variants as needed
    #[error("Unable to parse the file: {0}")]
    InvalidFile(String),
    #[error("Missing Required file")]
    MissingFile,
    #[error("Issues with IO")]
    IO,
    #[error("Issues with TCPListener")]
    TcpListenerError(std::io::Error),
    #[error("Missing command")]
    MissingCommandError,
}

pub type AdapterResult<T> = Result<T, MyAdapterError>;
