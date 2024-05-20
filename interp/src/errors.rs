use crate::values::Value;
use crate::{
    flatten::flat_ir::prelude::AssignedValue, utils::assignment_to_string,
};
use calyx_ir::{self as ir, Assignment, Id};
use calyx_utils::Error as CalyxError;
use rustyline::error::ReadlineError;
use thiserror::Error;

// Utility type
pub type InterpreterResult<T> = Result<T, BoxedInterpreterError>;

pub struct BoxedInterpreterError(Box<InterpreterError>);

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
        pest_consume::Error<crate::debugger::parser::command_parser::Rule>,
    ),

    /// Unable to parse the debugger command
    #[error(transparent)]
    MetadataParseError(
        #[from]
        pest_consume::Error<crate::debugger::source::metadata_parser::Rule>,
    ),
    // Unable to parse metadata
    #[error(transparent)]
    NewMetadataParseError(
        #[from] pest_consume::Error<crate::debugger::new_parser::Rule>,
    ),

    // Missing metadata
    #[error("missing metadata")]
    MissingMetaData,

    /// Wrapper for errors coming from the interactive CLI
    #[error(transparent)]
    ReadlineError(#[from] ReadlineError),

    /// An error for the exit command to the interactive debugger
    #[error("exit")]
    Exit,

    /// Wrapper error for parsing & related compiler errors
    #[error("{0:?}")]
    CompilerError(Box<CalyxError>),

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
    Please ensure that the dimensions of your input memories match their initialization data in the supplied data file")]
    IncorrectMemorySize {
        mem_dim: String,
        expected: u64,
        given: usize,
    },

    #[error("interpreter does not have an implementation of the \"{0}\" primitive. If the interpreter should have an implementation of this primitive please open a github issue or PR.")]
    UnknownPrimitive(String),
    #[error("program evaluated the truth value of a wire \"{}.{}\" which is not one bit. Wire is {} bits wide.", 0.0, 0.1, 1)]
    InvalidBoolCast((Id, Id), u64),
    #[error("the interpreter attempted to exit the group \"{0}\" before it finished. This should never happen, please report it.")]
    InvalidGroupExitNamed(Id),
    #[error("the interpreter attempted to exit a phantom group before it finished. This should never happen, please report it")]
    InvalidGroupExitUnnamed,

    #[error("invalid memory access to memory {}. Given index ({}) but memory has dimension ({})", name, access.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), dims.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))]
    InvalidMemoryAccess {
        access: Vec<u64>,
        dims: Vec<u64>,
        name: Id,
    },
    #[error("Both read and write signals provided to the sequential memory.")]
    SeqMemoryError,

    // TODO (Griffin): Make this error message better please
    #[error("Computation has under/overflowed its bounds")]
    OverflowError,

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    //TODO Griffin: Make this more descriptive
    #[error("Attempted to write an undefined value to register or memory")]
    UndefinedWrite,

    //TODO Griffin: Make this more descriptive
    #[error("Attempted to write an undefined memory address")]
    UndefinedWriteAddr,

    // TODO Griffin: Make this more descriptive
    #[error("Attempted to read an undefined memory address")]
    UndefinedReadAddr,

    #[error(transparent)]
    SerializationError(
        #[from] crate::serialization::data_dump::SerializationError,
    ),
}

impl InterpreterError {
    pub fn conflicting_assignments(
        port_id: Id,
        parent_id: Id,
        a1: &Assignment<ir::Nothing>,
        a2: &Assignment<ir::Nothing>,
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

impl From<CalyxError> for InterpreterError {
    fn from(e: CalyxError) -> Self {
        Self::CompilerError(Box::new(e))
    }
}

impl From<crate::structures::stk_env::CollisionError<*const ir::Port, Value>>
    for InterpreterError
{
    fn from(
        err: crate::structures::stk_env::CollisionError<
            *const calyx_ir::Port,
            crate::values::Value,
        >,
    ) -> Self {
        // when the error is first raised, the IR has not yet been deconstructed, so this
        // dereference is safe
        let port: &ir::Port = unsafe { &*err.0 };
        let parent_name = port.get_parent_name();
        let port_name = port.name;
        Self::ParOverlap {
            port_id: port_name,
            parent_id: parent_name,
            v1: err.1,
            v2: err.2,
        }
    }
}

impl From<std::str::Utf8Error> for InterpreterError {
    fn from(err: std::str::Utf8Error) -> Self {
        CalyxError::invalid_file(err.to_string()).into()
    }
}
