use crate::errors::Result;
use crate::{
    lang::{component, context},
    utils::OutputFile,
};
use pretty::termcolor::ColorSpec;
use pretty::RcDoc;

/// All backends must implement this trait.
/// `Backend::name` returns the name of this backend.
/// `Backend::validate` should return `Ok(())` if the
/// program is in the expected form and `Err(...)` otherwise.
/// `Backend::emit` should convert the program to a formted string
/// `Backend::run` is the composition of these two functions.
pub trait Backend {
    fn name() -> &'static str;
    fn validate(prog: &context::Context) -> Result<()>;
    fn emit(prog: &context::Context, write: OutputFile) -> Result<()>;
    fn run(prog: &context::Context, file: OutputFile) -> Result<()> {
        Self::validate(&prog)?;
        Self::emit(prog, file)
    }
}

pub trait Emitable {
    fn doc<'a>(
        &self,
        ctx: &context::Context,
        comp: &component::Component,
    ) -> Result<RcDoc<'a, ColorSpec>>;
}
