//! Internal representation for the [Calyx compiler](https://calyxir.org).
//!
//! The representation is generated from the frontend AST.
//! The key differences between the frontend AST and the IR are:
//! 1. The IR uses pointers instead of [`Id`] to refer to things like [`Port`] and
//!    [`Group`].
//! 2. The IR attempts to represent similar concepts in a homogeneous manner.

// Modules defining internal structures.
mod builder;
mod common;
mod component;
mod context;
mod control;
mod flat_guard;
mod guard;
mod printer;
mod reserved_names;
mod structure;

/// Modules to rewrite the IR
pub mod rewriter;

// Re-export types at the module level.
pub use builder::Builder;
pub use calyx_utils::{GetName, Id};
pub use common::{rrc, RRC, WRC};
pub use component::{Component, IdList};
pub use context::{BackendConf, Context};
pub use control::{
    Cloner, Control, Empty, Enable, GenericControl, If, Invoke, Par, Repeat,
    Seq, StaticControl, StaticEnable, StaticIf, StaticInvoke, StaticPar,
    StaticRepeat, StaticSeq, While,
};
pub use flat_guard::{FlatGuard, GuardPool, GuardRef};
pub use guard::{Guard, Nothing, PortComp, StaticTiming};
pub use printer::Printer;
pub use reserved_names::RESERVED_NAMES;
pub use rewriter::Rewriter;
pub use structure::{
    Assignment, Binding, Canonical, Cell, CellType, CombGroup, Group, Port,
    PortIterator, PortParent, StaticGroup,
};

// Re-export types from the frontend.
pub use calyx_frontend::{
    Attribute, Attributes, BoolAttr, Direction, GetAttributes, InternalAttr,
    LibrarySignatures, NumAttr, PortDef, Primitive, PrimitiveInfo, Width,
    DEPRECATED_ATTRIBUTES,
};

/// Module to transform AST programs into IR.
pub mod from_ast;

/// Convinience macros for constructing IR nodes.
mod macros;

/// Serializer methods for IR nodes.
pub mod serializers;

pub mod utils;
