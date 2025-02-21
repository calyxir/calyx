use crate::{
    flatten::{
        flat_ir::{
            base::{
                AssignmentIdx, AssignmentWinner, ComponentIdx, GlobalCellIdx,
                GlobalPortIdx,
            },
            prelude::AssignedValue,
        },
        structures::environment::{
            clock::{ClockError, ClockErrorWithCell},
            Environment,
        },
    },
    serialization::Shape,
};
use baa::BitVecOps;
use calyx_utils::{Error as CalyxError, MultiError as CalyxMultiError};
use itertools::Itertools;
use owo_colors::OwoColorize;
use rustyline::error::ReadlineError;
use thiserror::Error;

use std::fmt::Write;

/// A type alias for a result with an [BoxedCiderError] as the error type
pub type CiderResult<T> = Result<T, BoxedCiderError>;

/// A wrapper type for [InterpreterError]. This exists to allow a smaller return
/// size for results since the error type is large.
pub struct BoxedCiderError(Box<CiderError>);

impl BoxedCiderError {
    /// Get a mutable reference to the inner error
    pub fn inner_mut(&mut self) -> &mut CiderError {
        &mut self.0
    }
}

impl std::fmt::Display for BoxedCiderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&*self.0, f)
    }
}

impl std::fmt::Debug for BoxedCiderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl std::error::Error for BoxedCiderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl std::ops::Deref for BoxedCiderError {
    type Target = CiderError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for BoxedCiderError
where
    T: Into<CiderError>,
{
    fn from(e: T) -> Self {
        Self(Box::new(T::into(e)))
    }
}

/// An enum representing the different types of errors that can occur during
/// simulation and debugging
#[derive(Error)]
pub enum CiderError {
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

    #[error("{mem_dim} Memory given initialization data with invalid dimension.
    When flattened, expected {expected} entries, but the memory was supplied with {given} entries instead.
    Please ensure that the dimensions of your input memories match their initialization data in the supplied data file")]
    IncorrectMemorySize {
        mem_dim: String,
        expected: u64,
        given: usize,
    },

    /// A wrapper for IO errors
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// A wrapper for serialization errors
    #[error(transparent)]
    SerializationError(#[from] crate::serialization::SerializationError),

    /// A nonspecific error, used for arbitrary messages
    #[error("{0}")]
    GenericError(String),
}

pub type RuntimeResult<T> = Result<T, BoxedRuntimeError>;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct BoxedRuntimeError(#[from] Box<RuntimeError>);

impl<Inner: Into<RuntimeError>> From<Inner> for BoxedRuntimeError {
    fn from(value: Inner) -> Self {
        Self(Box::new(value.into()))
    }
}

impl BoxedRuntimeError {
    pub fn prettify_message<
        C: AsRef<crate::flatten::structures::context::Context> + Clone,
    >(
        self,
        env: &Environment<C>,
    ) -> CiderError {
        self.0.prettify_message(env)
    }
}

#[derive(Error, Debug)]
#[error(
    "conflicting assigns
        1. {a1}
        2. {a2}
    "
)]
pub struct ConflictingAssignments {
    pub target: GlobalPortIdx,
    pub a1: AssignedValue,
    pub a2: AssignedValue,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error(transparent)]
    ClockError(#[from] ClockErrorWithCell),

    #[error("Some guards are undefined: {0:?}")]
    UndefinedGuardError(
        Vec<(GlobalCellIdx, AssignmentIdx, Vec<GlobalPortIdx>)>,
    ),

    /// The error for attempting to write to an undefined memory address. This
    /// is distinct from writing to an out of bounds address.
    //TODO Griffin: Make this more descriptive
    #[error(
        "Attempted to write an undefined memory address in memory named \"{0:?}\""
    )]
    UndefinedWriteAddr(GlobalCellIdx),

    /// The error for attempting to read from an undefined memory address. This
    /// is distinct from reading from an out of bounds address.
    #[error(
        "Attempted to read an undefined memory address from memory named \"{0:?}\""
    )]
    UndefinedReadAddr(GlobalCellIdx),

    #[error("Attempted to undefine a defined port \"{0:?}\"")]
    UndefiningDefinedPort(GlobalPortIdx),

    /// The error for attempting to write `undef` values to a register or
    /// memory. Contains the name of the register or memory as a string
    //TODO Griffin: Make this more descriptive
    #[error(
        "Attempted to write an undefined value to register or memory named \"{0:?}\""
    )]
    UndefinedWrite(GlobalCellIdx),

    #[error("invalid memory access to memory. Given index ({}) but memory has dimension ", access.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))]
    InvalidMemoryAccess {
        access: Vec<u64>,
        dims: Shape,
        idx: GlobalCellIdx,
    },

    // TODO (Griffin): Make this error message better please
    #[error("Computation has under/overflowed its bounds")]
    OverflowError,

    #[error(transparent)]
    ConflictingAssignments(Box<ConflictingAssignments>),
}

// this is silly but needed to make the program print something sensible when returning
// a result from `main`
impl std::fmt::Debug for CiderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl From<CalyxError> for CiderError {
    fn from(e: CalyxError) -> Self {
        Self::CompilerError(Box::new(e))
    }
}

impl From<CalyxMultiError> for CiderError {
    fn from(e: CalyxMultiError) -> Self {
        Self::CompilerMultiError(Box::new(e))
    }
}

impl From<std::str::Utf8Error> for CiderError {
    fn from(err: std::str::Utf8Error) -> Self {
        CalyxError::invalid_file(err.to_string()).into()
    }
}

impl RuntimeError {
    pub fn prettify_message<
        C: AsRef<crate::flatten::structures::context::Context> + Clone,
    >(
        self,
        env: &Environment<C>,
    ) -> CiderError {
        fn assign_to_string<C: AsRef<crate::flatten::structures::context::Context> + Clone>(
            assign: &AssignedValue,
            env: &Environment<C>,
        ) -> (
            String,
            Option<(ComponentIdx, crate::flatten::flat_ir::component::AssignmentDefinitionLocation)>,
        ){
            match assign.winner() {
                AssignmentWinner::Cell => ("Cell".to_string(), None),
                AssignmentWinner::Implicit => ("Implicit".to_string(), None),
                AssignmentWinner::Assign(idx, _) => {
                    let (comp, loc) =
                        env.ctx().find_assignment_definition(*idx);

                    let str = env.ctx().printer().print_assignment(comp, *idx);
                    (str, Some((comp, loc)))
                }
            }
        }

        fn source_to_string<
            C: AsRef<crate::flatten::structures::context::Context> + Clone,
        >(
            source: &crate::flatten::flat_ir::component::AssignmentDefinitionLocation,
            comp: ComponentIdx,
            env: &Environment<C>,
        ) -> String {
            let comp_name = env.ctx().lookup_name(comp);
            match source {
                crate::flatten::flat_ir::component::AssignmentDefinitionLocation::CombGroup(g) => format!(" in comb group {comp_name}::{}", env.ctx().lookup_name(*g)),
                crate::flatten::flat_ir::component::AssignmentDefinitionLocation::Group(g) => format!(" in group {comp_name}::{}", env.ctx().lookup_name(*g)),
                crate::flatten::flat_ir::component::AssignmentDefinitionLocation::ContinuousAssignment => format!(" in {comp_name}'s continuous assignments"),
                //TODO Griffin: Improve the identification of the invoke
                crate::flatten::flat_ir::component::AssignmentDefinitionLocation::Invoke(_) => format!(" in an invoke in {comp_name}"),
            }
        }

        match self {
            RuntimeError::ConflictingAssignments(boxed_err) => {
                let ConflictingAssignments { target, a1, a2 } = *boxed_err;
                let (a1_str, a1_source) = assign_to_string(&a1, env);
                let (a2_str, a2_source) = assign_to_string(&a2, env);

                let a1_v = a1.val().to_bit_str();
                let a2_v = a2.val().to_bit_str();
                let a1_source = a1_source
                    .map(|(comp, s)| source_to_string(&s, comp, env))
                    .unwrap_or_default();
                let a2_source = a2_source
                    .map(|(comp, s)| source_to_string(&s, comp, env))
                    .unwrap_or_default();

                let target = env.get_full_name(target);

                CiderError::GenericError(
                    format!("conflicting assignments to port \"{target}\":\n 1. assigned {a1_v} by {a1_str}{a1_source}\n 2. assigned {a2_v} by {a2_str}{a2_source}")
                )
            }
            RuntimeError::UndefinedWrite(c) => CiderError::GenericError(format!("Attempted to write an undefined value to register or memory named \"{}\"", env.get_full_name(c))),
            RuntimeError::UndefinedWriteAddr(c) => CiderError::GenericError(format!("Attempted to write to an undefined memory address in memory named \"{}\"", env.get_full_name(c))),
            RuntimeError::UndefinedReadAddr(c) => CiderError::GenericError(format!("Attempted to read from an undefined memory address from memory named \"{}\"", env.get_full_name(c))),
            RuntimeError::ClockError(ClockErrorWithCell { error, cell, entry_number }) => {
                let race_location = if let Some(num) = entry_number {
                    format!("memory {} at entry {num}", env.get_full_name(cell))
                } else {
                    // register
                    format!("register {}", env.get_full_name(cell))
                };

                match error {
                    ClockError::ReadAfterWrite { write, read } => {
                        CiderError::GenericError(format!("Concurrent read & write to the same {race_location}\n  {}\n  {}", write.format(env), read.format(env)))
                    },
                    ClockError::WriteAfterWrite { write1, write2 } => {
                        CiderError::GenericError(format!("Concurrent writes to the same {race_location}\n  {}\n  {}", write1.format(env), write2.format(env)))
                    },
                    ClockError::WriteAfterRead { write , reads} => {
                        let plural_reads = reads.len() > 1;
                        let read_s = if plural_reads {"s"} else {""};
                        let formatted_reads = reads.iter().map(|r| r.format(env)).join("\n  ");

                        CiderError::GenericError(format!("Concurrent read{read_s} and write to the same {race_location}\n  {}\n  {formatted_reads}", write.format(env)))
                    },
                }

            }
            RuntimeError::UndefiningDefinedPort(p) => CiderError::GenericError(format!("Attempted to undefine a defined port \"{}\"", env.get_full_name(p))),
            RuntimeError::UndefinedGuardError(v) => {
                let mut message = String::from("Some guards contained undefined values after convergence:\n");
                for (cell, assign, ports) in v {
                    writeln!(message, "({}) in assignment {}", env.get_full_name(cell), env.ctx().printer().print_assignment(env.get_component_idx(cell).unwrap(), assign).bold()).unwrap();
                    for port in ports {
                        writeln!(message, "    {} is undefined", env.get_full_name(port).yellow()).unwrap();
                    }
                    writeln!(message).unwrap()
                }

                CiderError::GenericError(message)
            }
            RuntimeError::InvalidMemoryAccess { access, dims, idx } => {
                CiderError::GenericError(format!("Invalid memory access to memory named \"{}\". Given index ({}) but memory has dimension {}", env.get_full_name(idx), access.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "), dims.as_string()))
            },
            RuntimeError::OverflowError => todo!(),

        }
    }
}
