//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use itertools::Itertools;

use super::{Component, Id, Primitive};
use std::{collections::HashMap, path::PathBuf};

/// A representation of all the primitive definitions found while parsing
/// the root program.
#[derive(Debug, Default)]
pub struct LibrarySignatures {
    /// Direct mapping from name to primitives
    sigs: Vec<(PathBuf, HashMap<Id, Primitive>)>,
}

/// Iterator over primitive signatures defined in [LibrarySignatures].
pub struct SigIter<'a> {
    iter: Box<dyn Iterator<Item = &'a Primitive> + 'a>,
}

impl<'a> Iterator for SigIter<'a> {
    type Item = &'a Primitive;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl LibrarySignatures {
    /// Return the [Primitive] associated to this Id.
    pub fn find_primitive<S>(&self, name: S) -> Option<&Primitive>
    where
        S: AsRef<str>,
    {
        let key = Id::from(name.as_ref());
        for (_, sig) in &self.sigs {
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

    /// Return an iterator over the underlying
    pub fn signatures(&self) -> SigIter<'_> {
        SigIter {
            iter: Box::new(self.sigs.iter().flat_map(|(_, sig)| sig.values())),
        }
    }

    /// Return the underlying externs
    pub fn externs(self) -> Vec<(PathBuf, HashMap<Id, Primitive>)> {
        self.sigs
    }

    /// Return the paths for the extern defining files
    pub fn extern_paths(&self) -> Vec<&PathBuf> {
        self.sigs.iter().map(|(p, _)| p).collect_vec()
    }
}

impl From<Vec<(PathBuf, Vec<Primitive>)>> for LibrarySignatures {
    fn from(externs: Vec<(PathBuf, Vec<Primitive>)>) -> Self {
        let mut lib = LibrarySignatures::default();
        for (path, prims) in externs {
            let map: HashMap<_, _> =
                prims.into_iter().map(|p| (p.name.clone(), p)).collect();
            lib.sigs.push((path, map));
        }
        lib
    }
}

/// The IR Context
#[derive(Debug)]
pub struct Context {
    /// The components for this program.
    pub components: Vec<Component>,
    /// Library definitions imported by the program.
    pub lib: LibrarySignatures,
    /// Enables synthesis mode.
    pub synthesis_mode: bool,
    /// Enables verification checks.
    pub enable_verification: bool,
    /// Extra options provided to the command line. Interperted by individual
    /// passes
    pub extra_opts: Vec<String>,
}
