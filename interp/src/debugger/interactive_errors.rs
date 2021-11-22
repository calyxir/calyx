use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebuggerError {
    /// The debugger command needs a target
    #[error("command requires a target")]
    RequiresTarget,

    #[error("cannot find {0}")]
    CannotFind(String),
}
