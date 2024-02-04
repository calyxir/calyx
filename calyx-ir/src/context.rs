//! An IR context. This is the top-level object for an IR and contains all information
//! need to transform, lower, an emit a program.
//! Passes usually have transform/analyze the components in the IR.
use super::{Component, Id};
use calyx_frontend::LibrarySignatures;

/// Configuration information for the backends.
#[derive(Default)]
pub struct BackendConf {
    /// Enables synthesis mode.
    pub synthesis_mode: bool,
    /// Enables verification checks.
    pub enable_verification: bool,
    /// Use flat (ANF) assignments for guards instead of deep expression trees.
    pub flat_assign: bool,
    /// [FIRRTL backend only] Emit extmodule declarations for primtives
    /// for use with SystemVerilog implementations
    pub emit_primitive_extmodules: bool,
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

impl Context {
    // Return the index to the entrypoint component.
    fn entrypoint_idx(&self) -> usize {
        self.components
            .iter()
            .position(|c| c.name == self.entrypoint)
            .unwrap_or_else(|| panic!("No entrypoint in the program"))
    }

    /// Return the entrypoint component.
    pub fn entrypoint(&self) -> &Component {
        &self.components[self.entrypoint_idx()]
    }

    /// Return the entrypoint component with mutable access.
    pub fn entrypoint_mut(&mut self) -> &mut Component {
        let idx = self.entrypoint_idx();
        &mut self.components[idx]
    }
}
