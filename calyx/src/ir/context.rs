//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;

use super::{Component, Id, Primitive};
use std::path::PathBuf;

/// A representation of all the primitive definitions found while parsing
/// the root program.
#[derive(Debug, Default)]
pub struct LibrarySignatures {
    /// Direct mapping from name to primitives
    primitive_definitions: Vec<(PathBuf, LinkedHashMap<Id, Primitive>)>,
}

impl LibrarySignatures {
    /// Return the [Primitive] associated with the given name if defined, otherwise return None.
    pub fn find_primitive<S>(&self, name: S) -> Option<&Primitive>
    where
        S: AsRef<str>,
    {
        let key = Id::from(name.as_ref());
        for (_, sig) in &self.primitive_definitions {
            if let Some(p) = sig.get(&key) {
                return Some(p);
            }
        }
        None
    }

    /// Return the [Primitive] associated to this Id.
    pub fn get_primitive<S>(&self, name: S) -> &Primitive
    where
        S: AsRef<str>,
    {
        self.find_primitive(&name).unwrap_or_else(|| {
            panic!(
                "Primitive `{}` is not defined in the context.",
                name.as_ref()
            )
        })
    }

    /// Return an iterator over all defined primitives.
    pub fn signatures(&self) -> impl Iterator<Item = &Primitive> + '_ {
        self.primitive_definitions
            .iter()
            .flat_map(|(_, sig)| sig.values())
    }

    /// Return the underlying externs
    pub fn externs(self) -> Vec<(PathBuf, LinkedHashMap<Id, Primitive>)> {
        self.primitive_definitions
    }

    /// Return the paths for the extern defining files
    pub fn extern_paths(&self) -> Vec<&PathBuf> {
        self.primitive_definitions
            .iter()
            .map(|(p, _)| p)
            .collect_vec()
    }
}

impl<I> From<I> for LibrarySignatures
where
    I: IntoIterator<Item = (PathBuf, Vec<Primitive>)>,
{
    fn from(externs: I) -> Self {
        let mut lib = LibrarySignatures::default();
        for (path, prims) in externs {
            let map: LinkedHashMap<_, _> =
                prims.into_iter().map(|p| (p.name, p)).collect();
            lib.primitive_definitions.push((path, map));
        }
        lib
    }
}

/// Configuration information for the backends.
#[derive(Default)]
pub struct BackendConf {
    /// Enables synthesis mode.
    pub synthesis_mode: bool,
    /// Enables verification checks.
    pub enable_verification: bool,
    /// Generate initial assignments for input ports
    pub initialize_inputs: bool,
}

/// The IR Context that represents an entire Calyx program with all of its
/// imports and dependencies resolved.
pub struct Context {
    /// The components for this program.
    pub components: Vec<Component>,
    /// Library definitions imported by the program.
    pub lib: LibrarySignatures,
    /// Entrypoint for the program
    pub entrypoint: Id,
    /// Configuration flags for backends.
    pub bc: BackendConf,
    /// Extra options provided to the command line.
    /// Interpreted by individual passes
    pub extra_opts: Vec<String>,
    /// An optional opaque metadata string which is used by Cider
    pub metadata: Option<String>,
}
