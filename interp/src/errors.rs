use calyx::errors::Error;
use rustyline::error::ReadlineError;
use std::fmt::Debug;

// Utility type
pub type InterpreterResult<T> = Result<T, InterpreterError>;
pub enum InterpreterError {
    /// The given debugger command is invalid/malformed
    InvalidCommand(String),

    /// The given debugger command does not exist
    UnknownCommand(String),

    /// Wrapper for errors coming from the interactive CLI
    ReadlineError(ReadlineError),

    /// An error for an interrupt to the interactive debugger
    Interrupt,

    /// Wrapper error for parsing & related compiler errors
    CompilerError(Box<Error>),

    /// There is no main component in the given program
    MissingMainComponent,
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
            InterpreterError::MissingMainComponent => {
                write!(f, "Interpreter Error: There is no main component")
            }
            InterpreterError::Interrupt => {
                write!(f, "Interrupted")
            }
        }
    }
}

impl From<Error> for InterpreterError {
    fn from(e: Error) -> Self {
        Self::CompilerError(Box::new(e))
    }
}

impl From<ReadlineError> for InterpreterError {
    fn from(e: ReadlineError) -> Self {
        if let ReadlineError::Interrupted = e {
            InterpreterError::Interrupt
        } else {
            InterpreterError::ReadlineError(e)
        }
    }
}
