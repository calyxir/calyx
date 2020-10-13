//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use super::Component;
use crate::frontend::{ast, library};
use std::collections::HashMap;

/// The IR Context
#[derive(Debug)]
pub struct Context {
    /// The components for this program.
    pub components: Vec<Component>,
    /// Mapping from library functions to signatures
    pub lib_sigs: HashMap<ast::Id, library::ast::Primitive>,
    /// Enable debug mode logging.
    pub debug_mode: bool,
}
