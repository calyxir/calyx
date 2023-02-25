//! Interface for a Calyx backend.
use calyx_ir as ir;
use calyx_utils::{CalyxResult, OutputFile};

/// A backend for Calyx.
pub trait Backend {
    /// The name of this backend.
    fn name(&self) -> &'static str;
    /// Validate this program for emitting using this backend. Returns an
    /// Err(..) if the program has unexpected constructs.
    fn validate(prog: &ir::Context) -> CalyxResult<()>;
    /// Transforms the program into a formatted string representing a valid
    /// and write it to `write`.
    fn emit(prog: &ir::Context, write: &mut OutputFile) -> CalyxResult<()>;
    /// Link the extern collected while parsing the program.
    fn link_externs(
        prog: &ir::Context,
        write: &mut OutputFile,
    ) -> CalyxResult<()>;
    /// Convience function to validate and emit the program.
    fn run(&self, prog: ir::Context, mut file: OutputFile) -> CalyxResult<()> {
        Self::validate(&prog)?;
        Self::link_externs(&prog, &mut file)?;
        Self::emit(&prog, &mut file)
    }
}
