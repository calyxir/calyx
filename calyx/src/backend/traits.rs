use crate::{
    errors::Error, errors::FutilResult, frontend::library, ir,
    utils::OutputFile,
};
use itertools::Itertools;

/// A backend for FuTIL.
pub trait Backend {
    /// The name of this backend.
    fn name(&self) -> &'static str;
    /// Validate this program for emitting using this backend. Returns an
    /// Err(..) if the program has unexpected constructs.
    fn validate(prog: &ir::Context) -> FutilResult<()>;
    /// Transforms the program into a formatted string representing a valid
    /// and write it to `write`.
    fn emit_primitives(
        prog: Vec<&library::ast::Implementation>,
        write: &mut OutputFile,
    ) -> FutilResult<()>;
    /// Transforms the program into a formatted string representing a valid
    /// and write it to `write`.
    fn emit(prog: &ir::Context, write: &mut OutputFile) -> FutilResult<()>;
    /// Convience function to validate and emit the program.
    fn run(&self, prog: &ir::Context, mut file: OutputFile) -> FutilResult<()> {
        Self::validate(&prog)?;
        let primitives = prog
            .used_primitives()
            .into_iter()
            .sorted_by_key(|x| &x.name)
            .map(|x| {
                x.implementation.get(0).ok_or_else(|| {
                    Error::MissingImplementation(self.name(), x.name.clone())
                })
            })
            .collect::<FutilResult<_>>()?;
        Self::emit_primitives(primitives, &mut file)?;
        Self::emit(prog, &mut file)
    }
}
