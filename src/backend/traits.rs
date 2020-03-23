use crate::context;
use crate::errors;
use crate::lang::component;
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
    fn validate(prog: &context::Context) -> Result<(), errors::Error>;
    fn emit(prog: &context::Context) -> Result<(), errors::Error>;
    fn run(prog: &context::Context) -> Result<(), errors::Error> {
        Self::validate(&prog)?;
        Self::emit(prog)
    }
}

pub trait Emitable {
    fn doc<'a>(
        &'a self,
        comp: &'a component::Component,
    ) -> RcDoc<'a, ColorSpec>;
}
