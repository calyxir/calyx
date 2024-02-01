use crate::run;
use cranelift_entity::entity_impl;

/// A State is a type of file that Operations produce or consume.
pub struct State {
    /// The name of the state, for the UI.
    pub name: String,

    /// The file extensions that this state can be represented by.
    ///
    /// The first extension in the list is used when generating a new filename for the state. If
    /// the list is empty, this is a "pseudo-state" that doesn't correspond to an actual file.
    /// Pseudo-states can only be final outputs; they are appropraite for representing actions that
    /// interact directly with the user, for example.
    pub extensions: Vec<String>,
}

impl State {
    /// Check whether a filename extension indicates this state.
    pub fn ext_matches(&self, ext: &str) -> bool {
        self.extensions.iter().any(|e| e == ext)
    }

    /// Is this a "pseudo-state": doesn't correspond to an actual file, and must be an output state?
    pub fn is_pseudo(&self) -> bool {
        self.extensions.is_empty()
    }
}

/// A reference to a State.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct StateRef(u32);
entity_impl!(StateRef, "state");

/// An Operation transforms files from one State to another.
pub struct Operation {
    pub name: String,
    pub input: StateRef,
    pub output: StateRef,
    pub setups: Vec<SetupRef>,
    pub emit: Box<dyn run::EmitBuild>,
}

/// A reference to an Operation.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct OpRef(u32);
entity_impl!(OpRef, "op");

/// A Setup runs at configuration time and produces Ninja machinery for Operations.
pub struct Setup {
    pub name: String,
    pub emit: Box<dyn run::EmitSetup>,
}

/// A reference to a Setup.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SetupRef(u32);
entity_impl!(SetupRef, "setup");
