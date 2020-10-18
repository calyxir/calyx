//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use super::{Component, Id};
use crate::{frontend::library, ir};
use std::collections::HashMap;

/// The IR Context
#[derive(Debug)]
pub struct Context {
    /// The components for this program.
    pub components: Vec<Component>,
    /// Import statements for the file.
    pub import_statements: Vec<String>,
    /// Mapping from library functions to signatures
    pub lib_sigs: HashMap<Id, library::ast::Primitive>,
    /// Enable debug mode logging.
    pub debug_mode: bool,
}

impl Context {
    /// Returns the primitives that are used by this context.
    pub fn used_primitives(&self) -> Vec<&library::ast::Primitive> {
        let mut used = HashMap::new();
        for comp in &self.components {
            for cell in &comp.cells {
                if let ir::CellType::Primitive { name, .. } =
                    &cell.borrow().prototype
                {
                    used.insert(name.clone(), &self.lib_sigs[&name]);
                }
            }
        }
        used.drain().map(|(_, v)| v).collect()
    }
}
