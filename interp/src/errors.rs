use crate::flatten::flat_ir::prelude::AssignedValue;
use baa::BitVecValue;
use calyx_ir::Id;
use calyx_utils::{Error as CalyxError, MultiError as CalyxMultiError};
use rustyline::error::ReadlineError;
use thiserror::Error;

/// A type alias for a result with an [BoxedInterpreterError] as the error type
pub type InterpreterResult<T> = Result<T, BoxedInterpreterError>;

/// A wrapper type for [InterpreterError]. This exists to allow a smaller return
/// size for results since the error type is large.
pub struct BoxedInterpreterError(Box<InterpreterError>);

impl BoxedInterpreterError {
    /// Get a mutable reference to the inner error
    pub fn inner_mut(&mut self) -> &mut InterpreterError {
        &mut self.0
    }
}

impl std::fmt::Display for BoxedInterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&*self.0, f)
    }
}

impl std::fmt::Debug for BoxedInterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl std::error::Error for BoxedInterpreterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl std::ops::Deref for BoxedInterpreterError {
    type Target = InterpreterError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for BoxedInterpreterError
where
    T: Into<InterpreterError>,
{
    fn from(e: T) -> Self {
        Self(Box::new(T::into(e)))
    }
}

/// An enum representing the different types of errors that can occur during
/// simulation and debugging
#[derive(Error)]
pub enum InterpreterError {
    /// The given debugger command is invalid/malformed
    #[error("invalid command - {0}")]
    InvalidCommand(String),

    /// The given debugger command does not exist
    #[error("unknown command - {0}")]
    UnknownCommand(String),

    /// Unable to parse the debugger command
    #[error(transparent)]
    ParseError(
        #[from]
        pest_consume::Error<crate::debugger::commands::command_parser::Rule>,
    ),

    /// Unable to parse the debugger command
    #[error(transparent)]
    MetadataParseError(
        #[from]
        pest_consume::Error<crate::debugger::source::metadata_parser::Rule>,
    ),
    /// Unable to parse metadata
    #[error(transparent)]
    NewMetadataParseError(
        #[from] pest_consume::Error<crate::debugger::source::new_parser::Rule>,
    ),

    /// Metadata is unavailable
    #[error("missing metadata")]
    MissingMetaData,

    /// Wrapper for errors coming from the interactive CLI
    #[error(transparent)]
    ReadlineError(#[from] ReadlineError),

    /// Wrapper error for parsing & related compiler errors
    #[error("{0:?}")]
    CompilerError(Box<CalyxError>),

    /// Wrapper error for compiler multi errors
    #[error("{0:?}")]
    CompilerMultiError(Box<CalyxMultiError>),

    /// There is no main component in the given program
    #[error("no main component")]
    MissingMainComponent,

    #[error(
        "conflicting assigns
        1. {a1}
        2. {a2}
    "
    )]
    FlatConflictingAssignments {
        a1: AssignedValue,
        a2: AssignedValue,
    },

    /// A currently defunct error type for cross branch conflicts
    #[error(
        "par assignments not disjoint: {parent_id}.{port_id}
    1. {v1:?}
    2. {v2:?}"
    )]
    ParOverlap {
        port_id: Id,
        parent_id: Id,
        v1: BitVecValue,
        v2: BitVecValue,
    },

    #[error("{mem_dim} Memory given initialization data with invalid dimension.
    When flattened, expected {expected} entries, but the memory was supplied with {given} entries instead.
    Please ensure that the dimensions of your input memories match their initialization data in the supplied data file")]
    IncorrectMemorySize {
        mem_dim: String,
        expected: u64,
        given: usize,
    },

    #[error("invalid memory access to memory {}. Given index ({}) but memory has dimension ({})", name, access.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), dims.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))]
    InvalidMemoryAccess {
        access: Vec<u64>,
        dims: Vec<u64>,
        name: Id,
    },

    // TODO (Griffin): Make this error message better please
    #[error("Computation has under/overflowed its bounds")]
    OverflowError,

    /// A wrapper for IO errors
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// The error for attempting to write `undef` values to a register or
    /// memory. Contains the name of the register or memory as a string
    //TODO Griffin: Make this more descriptive
    #[error(
        "Attempted to write an undefined value to register or memory named \"{0}\""
    )]
    UndefinedWrite(String),

    /// The error for attempting to write to an undefined memory address. This
    /// is distinct from writing to an out of bounds address.
    //TODO Griffin: Make this more descriptive
    #[error(
        "Attempted to write an undefined memory address in memory named \"{0}\""
    )]
    UndefinedWriteAddr(String),

    /// The error for attempting to read from an undefined memory address. This
    /// is distinct from reading from an out of bounds address.
    #[error(
        "Attempted to read an undefined memory address from memory named \"{0}\""
    )]
    UndefinedReadAddr(String),

    /// A wrapper for serialization errors
    #[error(transparent)]
    SerializationError(#[from] crate::serialization::SerializationError),
}

// this is silly but needed to make the program print something sensible when returning
// a result from `main`
impl std::fmt::Debug for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl From<CalyxError> for InterpreterError {
    fn from(e: CalyxError) -> Self {
        Self::CompilerError(Box::new(e))
    }
}

impl From<CalyxMultiError> for InterpreterError {
    fn from(e: CalyxMultiError) -> Self {
        Self::CompilerMultiError(Box::new(e))
    }
}

impl From<std::str::Utf8Error> for InterpreterError {
    fn from(err: std::str::Utf8Error) -> Self {
        CalyxError::invalid_file(err.to_string()).into()
    }
}
