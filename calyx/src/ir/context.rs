//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.

use super::Component;

/// The IR Context
pub struct Context {
    /// The components for this program.
    components: Vec<Component>,
    /// Enable debug mode logging.
    debug_mode: bool,
}
