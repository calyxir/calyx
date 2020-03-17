use crate::context;
use crate::errors;
use crate::lang::component;
use pretty::RcDoc;

/// All backends must implement this trait.
/// `Backend::validate` should return `Ok(())` if the
/// program is in the expected form and `Err(...)` otherwise.
/// `Backend::to_string` should convert the program to a formted string
/// `Backend::emit` is the composition of these two functions.
pub trait Backend {
    fn validate(prog: &context::Context) -> Result<(), errors::Error>;
    fn to_string(prog: &context::Context) -> String;
    fn emit(prog: context::Context) -> Result<String, errors::Error> {
        Self::validate(&prog)?;
        Ok(Self::to_string(&prog))
    }
}

pub trait Emitable {
    fn doc<'a>(&'a self, comp: &'a component::Component) -> RcDoc<'a>;
}
