use crate::utils::assignment_to_string;
use calyx::errors::Error;
use calyx::ir::{Assignment, Id};
use rustyline::error::ReadlineError;
use thiserror::Error;

// Utility type
pub type InterpreterResult<T> = Result<T, InterpreterError>;

#[derive(Error)]
pub enum InterpreterError {
    /// The given debugger command is invalid/malformed
    #[error("invalid command - {0}")]
    InvalidCommand(String),

    /// The given debugger command does not exist
    #[error("unknown command - {0}")]
    UnknownCommand(String),

    /// Wrapper for errors coming from the interactive CLI
    #[error(transparent)]
    ReadlineError(#[from] ReadlineError),

    /// An error for the exit command to the interactive debugger
    #[error("exit")]
    Exit,

    /// Wrapper error for parsing & related compiler errors
    #[error("{0:?}")]
    CompilerError(Box<Error>),

    /// There is no main component in the given program
    #[error("no main component")]
    MissingMainComponent,

    /// Multiple assignments conflicting during interpretation
    #[error(
        "multiple assignments to one port: {parent_id}.{port_id}
    Conflict between:
     1. {a1}
     2. {a2}"
    )]
    ConflictingAssignments {
        port_id: Id,
        parent_id: Id,
        a1: String,
        a2: String,
    },
}

impl InterpreterError {
    pub fn conflicting_assignments(
        port_id: Id,
        parent_id: Id,
        a1: &Assignment,
        a2: &Assignment,
    ) -> Self {
        Self::ConflictingAssignments {
            port_id,
            parent_id,
            a1: assignment_to_string(a1),
            a2: assignment_to_string(a2),
        }
    }
}

// this is silly but needed to make the program print something sensible when returning
// a result from `main`
impl std::fmt::Debug for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl From<Error> for InterpreterError {
    fn from(e: Error) -> Self {
        Self::CompilerError(Box::new(e))
    }
}
