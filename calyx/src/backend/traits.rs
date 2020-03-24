use crate::errors;
use crate::lang::{component, context};
use pretty::termcolor::ColorSpec;
use pretty::RcDoc;
use std::io::Write;

/// All backends must implement this trait.
/// `Backend::name` returns the name of this backend.
/// `Backend::validate` should return `Ok(())` if the
/// program is in the expected form and `Err(...)` otherwise.
/// `Backend::emit` should convert the program to a formted string
/// `Backend::run` is the composition of these two functions.
pub trait Backend {
    fn name() -> &'static str;
    fn validate(prog: &context::Context) -> Result<(), errors::Error>;
    fn emit<W: Write>(
        prog: &context::Context,
        write: W,
    ) -> Result<(), errors::Error>;
    fn run<W: Write>(
        prog: &context::Context,
        file: W,
    ) -> Result<(), errors::Error> {
        Self::validate(&prog)?;
        Self::emit(prog, file)
    }
}

pub trait Emitable {
    fn doc<'a>(
        &'a self,
        comp: &'a component::Component,
    ) -> RcDoc<'a, ColorSpec>;
}
