#[allow(dead_code)] //remove this later
#[derive(thiserror::Error, Debug)]
pub enum MyAdapterError {
    #[error("Unhandled command")]
    UnhandledCommandError,
    #[error("Unable to parse the file: {0}")]
    InvalidFile(String),
    #[error("Missing Required file")]
    MissingFile,
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Missing command")]
    MissingCommandError,
}
pub type AdapterResult<T> = Result<T, MyAdapterError>;
