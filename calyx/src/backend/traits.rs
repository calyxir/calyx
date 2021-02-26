//! Interface for a Calyx backend.
use crate::{errors::FutilResult, ir, utils::OutputFile};

/// A backend for Calyx.
pub trait Backend {
    /// The name of this backend.
    fn name(&self) -> &'static str;
    /// Validate this program for emitting using this backend. Returns an
    /// Err(..) if the program has unexpected constructs.
    fn validate(prog: &ir::Context) -> FutilResult<()>;
    /// Transforms the program into a formatted string representing a valid
    /// and write it to `write`.
    fn emit(prog: &ir::Context, write: &mut OutputFile) -> FutilResult<()>;
    /// Link the extern collected while parsing the program.
    fn link_externs(
        prog: &ir::Context,
        write: &mut OutputFile,
    ) -> FutilResult<()>;
    /// Convience function to validate and emit the program.
    fn run(&self, prog: &ir::Context, mut file: OutputFile) -> FutilResult<()> {
        Self::validate(&prog)?;
        Self::link_externs(&prog, &mut file)?;
        Self::emit(prog, &mut file)
    }
}
