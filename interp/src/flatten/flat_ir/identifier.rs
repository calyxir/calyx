use ahash::{HashMap, HashMapExt};
use std::hash::Hash;

use crate::flatten::structures::index_trait::{impl_index, IndexRef};

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, PartialOrd, Ord)]
pub struct Identifier(u32);

impl_index!(Identifier);

impl Identifier {
    #[inline]
    pub(crate) fn get_default_id() -> Identifier {
        // manually construct
        Identifier(0)
    }

    /// Utility method to resolve the string associated with an identifier
    pub fn resolve<'a>(&self, resolver: &'a IdMap) -> &'a String {
        resolver.lookup_string(self).unwrap()
    }
}

/// Internal enum to distinguish between the different parents for a port. Used
/// to format names appropriately
pub enum NameType {
    /// This is one of the component ports
    Interface,
    /// This port belongs to a group (go/done)
    Group,
    /// The standard port on a cell
    Cell,
}

pub enum CanonicalIdentifier {
    Standard {
        parent: Identifier,
        name: Identifier,
        name_type: NameType,
    },
    /// A constant literal
    Literal { width: u64, val: u64 },
}

impl CanonicalIdentifier {
    pub fn interface_port(parent: Identifier, name: Identifier) -> Self {
        Self::Standard {
            parent,
            name,
            name_type: NameType::Interface,
        }
    }

    pub fn group_port(parent: Identifier, name: Identifier) -> Self {
        Self::Standard {
            parent,
            name,
            name_type: NameType::Group,
        }
    }

    pub fn cell_port(parent: Identifier, name: Identifier) -> Self {
        Self::Standard {
            parent,
            name,
            name_type: NameType::Cell,
        }
    }

    pub fn literal(width: u64, val: u64) -> Self {
        Self::Literal { width, val }
    }

    pub fn format_name(&self, resolver: &IdMap) -> String {
        match self {
            CanonicalIdentifier::Standard {
                parent,
                name,
                name_type,
            } => {
                let parent = resolver.lookup_string(parent).unwrap();
                let port = resolver.lookup_string(name).unwrap();
                match name_type {
                    NameType::Interface => port.to_string(),
                    NameType::Group => format!("{}[{}]", parent, port),
                    NameType::Cell => format!("{}.{}", parent, port),
                }
            }
            CanonicalIdentifier::Literal { width, val } => {
                format!("{width}'d{val}")
            }
        }
    }

    pub fn name(&self) -> Option<&Identifier> {
        match self {
            CanonicalIdentifier::Standard { name, .. } => name.into(),
            CanonicalIdentifier::Literal { .. } => None,
        }
    }
}

/// A map used to store strings and assign them unique identifiers. This is
/// designed to work both forward and backwards. The forward map is a hashmap
/// while the backward map is a simple dense vector
///
/// This is using the [ahash] crate instead of the std
/// [HashMap](std::collections::HashMap) for general speed though that is likely
/// unnecessary as this should not be on any hot paths. If we want to be
/// resistant to hash attacks the forward map can be changed to be amenable to
/// that though we're generating with randomness so that is unlikely to be an
/// issue
#[derive(Debug)]
pub struct IdMap {
    /// The forward map hashes strings to identifiers
    forward: HashMap<String, Identifier>,
    /// Since identifiers are handed out linearly the backwards map is a vector
    /// of strings
    backward: Vec<String>,
}

impl IdMap {
    /// number of strings that are included by default. Used when constructing a
    /// table with a specific capacity
    const PREALLOCATED: usize = 3;

    /// inner builder style utility function
    fn insert_basic_strings(mut self) -> Self {
        self.insert("");
        self.insert("go");
        self.insert("done");
        self
    }

    /// Initializes a new identifier map with the empty string pre-inserted
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Initializes a new identifier map with the given number of slots
    /// preallocated and the empty string inserted
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            forward: HashMap::with_capacity(capacity + Self::PREALLOCATED),
            backward: Vec::with_capacity(capacity + Self::PREALLOCATED),
        }
        .insert_basic_strings()
    }

    /// Inserts a string mapping into the table and returns the identifier.
    /// If already present, the original identifier is returned
    pub fn insert<S>(&mut self, input: S) -> Identifier
    where
        S: AsRef<str>,
    {
        let id = self
            .forward
            .entry(input.as_ref().to_string())
            .or_insert_with_key(|k| {
                let id = Identifier::from(self.backward.len());
                self.backward.push(k.clone());
                id
            });

        *id
    }

    /// Returns the identifier associated with the string, if present
    pub fn lookup_id<S: AsRef<str>>(&self, key: S) -> Option<&Identifier> {
        self.forward.get(key.as_ref())
    }

    /// Returns the string associated with the identifier, if present
    pub fn lookup_string(&self, id: &Identifier) -> Option<&String> {
        self.backward.get(id.index())
    }
}

impl Default for IdMap {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
