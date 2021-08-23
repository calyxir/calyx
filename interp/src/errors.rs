use calyx::errors::Error;
use rustyline::error::ReadlineError;
use std::fmt::Debug;

// Utility type
pub type InterpreterResult<T> = Result<T, InterpreterError>;
pub enum InterpreterError {
    InvalidCommand(String), // this isn't used yet, but may be useful later when commands have more syntax
    UnknownCommand(String),
    ReadlineError(ReadlineError),
    CompilerError(Error),
}

impl Debug for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::InvalidCommand(msg) => {
                write!(f, "Invalid Command: {}", msg)
            }
            InterpreterError::UnknownCommand(s) => {
                write!(f, "Unknown command {}", s)
            }
            InterpreterError::ReadlineError(e) => {
                write!(f, "Failed to read from command line: {}", e)
            }
            InterpreterError::CompilerError(err) => {
                write!(f, "{:?}", err)
            }
        }
    }
}

impl From<Error> for InterpreterError {
    fn from(e: Error) -> Self {
        Self::CompilerError(e)
    }
}

impl From<ReadlineError> for InterpreterError {
    fn from(e: ReadlineError) -> Self {
        InterpreterError::ReadlineError(e)
    }
}
