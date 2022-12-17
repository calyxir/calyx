use crate::utils::GSym;
use serde::{Deserialize, Serialize};

/// Represents an identifier in a Calyx program
#[derive(Clone, Deserialize, Hash, PartialEq, Eq, Debug, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Id {
    pub id: GSym,
}

impl Id {
    pub fn new<S: ToString>(id: S) -> Self {
        Self {
            id: GSym::from(id.to_string()),
        }
    }
}

/* =================== Impls for Id to make them easier to use ============== */

impl Default for Id {
    fn default() -> Self {
        Id::new("")
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
        Id::new(s)
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id::new(s)
    }
}

impl PartialEq<GSym> for Id {
    fn eq(&self, other: &GSym) -> bool {
        self.id == *other
    }
}
impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == GSym::from(other)
    }
}
impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        self.id == GSym::from(*other)
    }
}
impl PartialEq<&Id> for Id {
    fn eq(&self, other: &&Id) -> bool {
        self.id == other.id
    }
}
impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        self.id == GSym::from(other)
    }
}

impl From<Id> for GSym {
    fn from(id: Id) -> Self {
        id.id
    }
}

impl From<&Id> for GSym {
    fn from(id: &Id) -> Self {
        id.id
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
