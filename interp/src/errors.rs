use calyx::errors::Error;
use rustyline::error::ReadlineError;
use thiserror::Error;

// Utility type
pub type InterpreterResult<T> = Result<T, InterpreterError>;

#[derive(Error, Debug)]
pub enum InterpreterError {
    /// The given debugger command is invalid/malformed
    #[error("invalid command - {0}")]
    InvalidCommand(String),

    /// The given debugger command does not exist
    #[error("unknown command - {0}")]
    UnknownCommand(String),

    /// Wrapper for errors coming from the interactive CLI
    #[error(transparent)]
    ReadlineError(ReadlineError),

    /// An error for an interrupt to the interactive debugger
    #[error("interrupted")]
    Interrupted,

    /// Wrapper error for parsing & related compiler errors
    #[error("{0:?}")]
    CompilerError(Box<Error>),

    /// There is no main component in the given program
    #[error("no main component")]
    MissingMainComponent,
}

impl From<Error> for InterpreterError {
    fn from(e: Error) -> Self {
        Self::CompilerError(Box::new(e))
    }
}

impl From<ReadlineError> for InterpreterError {
    fn from(e: ReadlineError) -> Self {
        if let ReadlineError::Interrupted = e {
            InterpreterError::Interrupted
        } else {
            InterpreterError::ReadlineError(e)
        }
    }
}
