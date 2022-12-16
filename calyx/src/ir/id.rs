use crate::errors::{Span, WithPos};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use symbol_table::GlobalSymbol;

/// Represents an identifier in a Calyx program
#[derive(Derivative, Clone, Deserialize)]
#[derivative(Hash, Eq, Debug, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Id {
    pub id: GlobalSymbol,
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialOrd = "ignore")]
    #[derivative(Ord = "ignore")]
    #[serde(skip)]
    span: Option<Rc<Span>>,
}

impl Id {
    pub fn new<S: ToString>(id: S, span: Option<Rc<Span>>) -> Self {
        Self {
            id: GlobalSymbol::from(id.to_string()),
            span,
        }
    }
}

impl WithPos for Id {
    fn copy_span(&self) -> Option<Rc<Span>> {
        self.span.clone()
    }
}

/* =================== Impls for Id to make them easier to use ============== */

impl Default for Id {
    fn default() -> Self {
        Id::new("", None)
    }
}

impl std::fmt::Display for Id {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.id)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.id.as_str()
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id::new(s, None)
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id::new(s, None)
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == GlobalSymbol::from(other)
    }
}

impl<S: AsRef<str>> PartialEq<S> for Id {
    fn eq(&self, other: &S) -> bool {
        self.id == GlobalSymbol::from(other.as_ref())
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.id.serialize(serializer)
    }
}
