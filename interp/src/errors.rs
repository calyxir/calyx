use crate::utils::assignment_to_string;
use crate::values::Value;
use calyx::errors::Error;
use calyx::ir::{self, Assignment, Id};
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

    #[error("unable to find component named \"{0}\"")]
    UnknownComponent(String),

    #[error(
        "par assignments not disjoint: {parent_id}.{port_id}
    1. {v1}
    2. {v2}"
    )]
    ParOverlap {
        port_id: Id,
        parent_id: Id,
        v1: Value,
        v2: Value,
    },
    #[error("invalid internal seq state. This should never happen, please report it")]
    InvalidSeqState,
    #[error(
        "invalid internal if state. This should never happen, please report it"
    )]
    InvalidIfState,
    #[error("invalid internal while state. This should never happen, please report it")]
    InvalidWhileState,

    #[error("{mem_dim} Memory given initialization data with invalid dimension.
    When flattened, expected {expected} entries, but the memory was supplied with {given} entries instead.
    Please ensure that the dimensions of your input memories match their initalization data in the supplied data file")]
    IncorrectMemorySize {
        mem_dim: String,
        expected: u64,
        given: usize,
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

impl From<crate::stk_env::CollisionError<*const ir::Port, Value>>
    for InterpreterError
{
    fn from(
        err: crate::stk_env::CollisionError<
            *const calyx::ir::Port,
            crate::values::Value,
        >,
    ) -> Self {
        // when the error is first raised, the IR has not yet been deconstructed, so this
        // dereference is safe
        let port: &ir::Port = unsafe { &*err.0 };
        let parent_name = port.get_parent_name();
        let port_name = port.name.clone();
        Self::ParOverlap {
            port_id: port_name,
            parent_id: parent_name,
            v1: err.1,
            v2: err.2,
        }
    }
}
