//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use super::{Component, Id};
use crate::frontend::ast;
use std::collections::HashMap;

/// A representation of all the primitive definitions found while parsing
/// the root program.
#[derive(Debug, Default)]
pub struct LibrarySignatures {
    /// Direct mapping from name to primitives
    sigs: HashMap<Id, ast::Primitive>,
    /// Paths to files that define externs (relative to the root file).
    pub paths: Vec<String>,
}

impl LibrarySignatures {
    /// Return the `Primitive` associated to this Id.
    pub fn find_primitive<S>(&self, name: S) -> Option<&ast::Primitive>
    where
        S: AsRef<str>,
    {
        self.sigs.get(&Id::from(name.as_ref()))
    }

    /// Return the `Primitive` associated to this Id.
    pub fn get_primitive<S>(&self, name: S) -> &ast::Primitive
    where
        S: AsRef<str>,
    {
        &self.sigs[&Id::from(name.as_ref())]
    }
}

impl From<Vec<(String, Vec<ast::Primitive>)>> for LibrarySignatures {
    fn from(externs: Vec<(String, Vec<ast::Primitive>)>) -> Self {
        let mut lib = LibrarySignatures::default();
        for (path, prims) in externs {
            lib.sigs
                .extend(prims.into_iter().map(|p| (p.name.clone(), p)));
            lib.paths.push(path);
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
    /// Enable debug mode logging.
    pub debug_mode: bool,
    /// Enables synthesis mode.
    pub synthesis_mode: bool,
    /// Original import statements.
    pub imports: Vec<String>,
}
