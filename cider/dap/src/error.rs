use dap::errors::ServerError;
use dap::requests::Command;

#[allow(dead_code)] // remove this later
#[derive(thiserror::Error)]
pub enum MyAdapterError {
    /// Represents an unhandled command error.
    #[error("Unhandled command: {0:?}")]
    UnhandledCommandError(Command),

    /// Represents an error when unable to parse the file.
    #[error("Unable to parse the file: {0}")]
    InvalidFile(String),

    /// Represents an error when a required file is missing.
    #[error("Missing Required file")]
    MissingFile,

    /// Represents an I/O error.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    /// Represents an error for an invalid path.
    #[error("Invalid path provided")]
    InvalidPathError,

    /// Represents an error when a command is missing.
    #[error("Missing command")]
    MissingCommandError,

    /// Represents a missing request.
    #[error("Missing request")]
    MissingRequest,

    /// Represents a server error.
    #[error(transparent)]
    ServerError(#[from] ServerError),
}

// Needed to properly display messages in output
impl std::fmt::Debug for MyAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

/// A type alias for the result returned by the adapter functions.
pub type AdapterResult<T> = Result<T, MyAdapterError>;
