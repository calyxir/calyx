use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebuggerError {
    #[error("cannot find {0}")]
    CannotFind(String),
}
