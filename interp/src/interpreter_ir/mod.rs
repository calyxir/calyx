//! A read-only variant of the IR used for more ergonomic interpretation.
//!
//! This module exposes alternate definitions for the core Calyx IR structures. The
//! primary reason for this duplications is that the interpreter needs to be able to
//! take apart control structures in a modular way without destroying the actual
//! structure. These variants allow subtrees of the control structure to be held without
//! causing lifetime issues with other components of the interpreter. As an auxillary
//! effect this enables (relatively) cheap cloning for control structures as they are
//! only needed in a read-only capacity.

mod component;
mod control;

pub use component::Component;
pub use control::{Control, Empty, Enable, If, Invoke, Par, Seq, While};

use std::rc::Rc;
pub type ComponentCtx = Rc<Vec<Rc<component::Component>>>;
pub type ContinuousAssignments =
    Rc<Vec<calyx_ir::Assignment<calyx_ir::Nothing>>>;
