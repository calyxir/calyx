use thiserror::Error;

#[allow(dead_code)] //remove this later
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
    #[error("Issues with TCPListener: {0}")]
    TcpListenerError(std::io::Error),
    #[error("Missing command")]
    MissingCommandError,
}

pub type AdapterResult<T> = Result<T, MyAdapterError>;
