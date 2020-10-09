use crate::errors::Span;
use derivative::Derivative;

/// Represents an identifier in a Futil program
#[derive(Derivative, Clone, PartialOrd, Ord, Debug)]
#[derivative(Hash, Eq, PartialEq)]
pub struct Id {
    pub id: String,
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    #[derivative(Debug = "ignore")]
    span: Option<Span>,
}

