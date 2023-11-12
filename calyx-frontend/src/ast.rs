//! Abstract Syntax Tree for Calyx
use super::parser;
use crate::{Attributes, PortDef, Primitive};
use atty::Stream;
use calyx_utils::{CalyxResult, Error, GPosIdx, Id};
use std::{num::NonZeroU64, path::PathBuf};

/// Corresponds to an individual Calyx file.
#[derive(Debug)]
pub struct NamespaceDef {
    /// Path to extern files.
    pub imports: Vec<String>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
    /// Extern statements and any primitive declarations in them.
    pub externs: Vec<(Option<String>, Vec<Primitive>)>,
    /// Optional opaque metadata
    pub metadata: Option<String>,
}

impl NamespaceDef {
    /// Construct a namespace from a file or the input stream.
    /// If no file is provided, the input stream must be a TTY.
    pub fn construct(file: &Option<PathBuf>) -> CalyxResult<Self> {
        match file {
            Some(file) => parser::CalyxParser::parse_file(file),
            None => {
                if atty::isnt(Stream::Stdin) {
                    parser::CalyxParser::parse(std::io::stdin())
                } else {
                    Err(Error::invalid_file(
                        "No file provided and terminal not a TTY".to_string(),
                    ))
                }
            }
        }
    }

    /// Construct a namespace from a definition using a string.
    pub fn construct_from_str(inp: &str) -> CalyxResult<Self> {
        parser::CalyxParser::parse(inp.as_bytes())
    }
}

/// AST statement for defining components.
#[derive(Debug)]
pub struct ComponentDef {
    /// Name of the component.
    pub name: Id,
    /// Defines input and output ports along with their attributes.
    pub signature: Vec<PortDef<u64>>,
    /// List of instantiated sub-components
    pub cells: Vec<Cell>,
    /// List of groups
    pub groups: Vec<Group>,
    /// List of StaticGroups
    pub static_groups: Vec<StaticGroup>,
    /// List of continuous assignments
    pub continuous_assignments: Vec<Wire>,
    /// Single control statement for this component.
    pub control: Control,
    /// Attributes attached to this component
    pub attributes: Attributes,
    /// True iff this is a combinational component
    pub is_comb: bool,
    /// (Optional) latency of component, if it is static
    pub latency: Option<NonZeroU64>,
}

impl ComponentDef {
    pub fn new<S>(
        name: S,
        is_comb: bool,
        latency: Option<NonZeroU64>,
        signature: Vec<PortDef<u64>>,
    ) -> Self
    where
        S: Into<Id>,
    {
        Self {
            name: name.into(),
            signature,
            cells: Vec::new(),
            groups: Vec::new(),
            static_groups: Vec::new(),
            continuous_assignments: Vec::new(),
            control: Control::empty(),
            attributes: Attributes::default(),
            is_comb,
            latency,
        }
    }
}

/// Statement that refers to a port on a subcomponent.
/// This is distinct from a `Portdef` which defines a port.
#[derive(Debug)]
pub enum Port {
    /// Refers to the port named `port` on the subcomponent
    /// `component`.
    Comp { component: Id, port: Id },

    /// Refers to the port named `port` on the component
    /// currently being defined.
    This { port: Id },

    /// `group[name]` parses into `Hole { group, name }`
    /// and is a hole named `name` on group `group`
    Hole { group: Id, name: Id },
}

// ===================================
// AST for wire guard expressions
// ===================================

#[derive(Debug)]
pub enum NumType {
    Decimal,
    Binary,
    Octal,
    Hex,
}

/// Custom bitwidth numbers
#[derive(Debug)]
pub struct BitNum {
    pub width: u64,
    pub num_type: NumType,
    pub val: u64,
    pub span: GPosIdx,
}

/// Atomic operations used in guard conditions and RHS of the
/// guarded assignments.
#[derive(Debug)]
pub enum Atom {
    /// Accessing a particular port on a component.
    Port(Port),
    /// A constant.
    Num(BitNum),
}

/// The AST for GuardExprs
#[derive(Debug)]
pub enum GuardExpr {
    // Logical operations
    And(Box<GuardExpr>, Box<GuardExpr>),
    Or(Box<GuardExpr>, Box<GuardExpr>),
    Not(Box<GuardExpr>),
    CompOp(CompGuard),
    Atom(Atom),
}

/// Guard Comparison Type
pub type CompGuard = (GuardComp, Atom, Atom);

/// The AST for StaticGuardExprs
#[derive(Debug)]
pub enum StaticGuardExpr {
    And(Box<StaticGuardExpr>, Box<StaticGuardExpr>),
    Or(Box<StaticGuardExpr>, Box<StaticGuardExpr>),
    Not(Box<StaticGuardExpr>),
    CompOp(CompGuard),
    Atom(Atom),
    StaticInfo((u64, u64)),
}

/// Possible comparison operators for guards.
#[derive(Debug)]
pub enum GuardComp {
    Eq,
    Neq,
    Gt,
    Lt,
    Geq,
    Leq,
}

/// Guards `expr` using the optional guard condition `guard`.
#[derive(Debug)]
pub struct Guard {
    pub guard: Option<GuardExpr>,
    pub expr: Atom,
}

/// Guards `expr` using the optional guard condition `guard`.
#[derive(Debug)]
pub struct StaticGuard {
    pub guard: Option<StaticGuardExpr>,
    pub expr: Atom,
}

// ===================================
// Data definitions for Structure
// ===================================

/// Prototype of the cell definition
#[derive(Debug)]
pub struct Proto {
    /// Name of the primitive.
    pub name: Id,
    /// Parameter binding for primitives
    pub params: Vec<u64>,
}

/// The Cell AST nodes.
#[derive(Debug)]
pub struct Cell {
    /// Name of the cell.
    pub name: Id,
    /// Name of the prototype this cell was built from.
    pub prototype: Proto,
    /// Attributes attached to this cell definition
    pub attributes: Attributes,
    /// Whether this cell is external
    pub reference: bool,
}

/// Methods for constructing the structure AST nodes.
impl Cell {
    /// Constructs a primitive cell instantiation.
    pub fn from(
        name: Id,
        proto: Id,
        params: Vec<u64>,
        attributes: Attributes,
        reference: bool,
    ) -> Cell {
        Cell {
            name,
            prototype: Proto {
                name: proto,
                params,
            },
            attributes,
            reference,
        }
    }
}

#[derive(Debug)]
pub struct Group {
    pub name: Id,
    pub wires: Vec<Wire>,
    pub attributes: Attributes,
    pub is_comb: bool,
}

#[derive(Debug)]
pub struct StaticGroup {
    pub name: Id,
    pub wires: Vec<StaticWire>,
    pub attributes: Attributes,
    pub latency: NonZeroU64,
}

/// Data for the `->` structure statement.
#[derive(Debug)]
pub struct Wire {
    /// Source of the wire.
    pub src: Guard,

    /// Guarded destinations of the wire.
    pub dest: Port,

    /// Attributes for this assignment
    pub attributes: Attributes,
}

/// Data for the `->` structure statement.
#[derive(Debug)]
pub struct StaticWire {
    /// Source of the wire.
    pub src: StaticGuard,

    /// Guarded destinations of the wire.
    pub dest: Port,

    /// Attributes for this assignment
    pub attributes: Attributes,
}

/// Control AST nodes.
/// Since enables and static enables are indistinguishable to the AST, there
/// is single Control Enum for both Static and Dynamic Control
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq {
        /// List of `Control` statements to run in sequence.
        stmts: Vec<Control>,
        /// Attributes
        attributes: Attributes,
    },
    /// Represents parallel composition of control statements.
    Par {
        /// List of `Control` statements to run in sequence.
        stmts: Vec<Control>,
        /// Attributes
        attributes: Attributes,
    },
    /// Standard imperative if statement
    If {
        /// Port that connects the conditional check.
        port: Port,

        /// Modules that need to be enabled to send signal on `port`.
        cond: Option<Id>,

        /// Control for the true branch.
        tbranch: Box<Control>,

        /// Control for the true branch.
        fbranch: Box<Control>,

        /// Attributes
        attributes: Attributes,
    },
    /// Standard imperative while statement
    While {
        /// Port that connects the conditional check.
        port: Port,

        /// Modules that need to be enabled to send signal on `port`.
        cond: Option<Id>,

        /// Control for the loop body.
        body: Box<Control>,

        /// Attributes
        attributes: Attributes,
    },
    /// Static Repeat (essentially a bounded while loop w/o a condition)
    Repeat {
        /// Control for the true branch.
        num_repeats: u64,

        /// Control for the true branch.
        body: Box<Control>,

        /// Attributes
        attributes: Attributes,
    },
    /// Runs the control for a list of subcomponents.
    Enable {
        /// Group to be enabled
        comp: Id,
        /// Attributes
        attributes: Attributes,
    },
    /// Invoke component with input/output assignments.
    Invoke {
        /// Name of the component to be invoked.
        comp: Id,
        /// Input assignments
        inputs: Vec<(Id, Atom)>,
        /// Output assignments
        outputs: Vec<(Id, Atom)>,
        /// Attributes
        attributes: Attributes,
        /// Combinational group that may execute with this invoke.
        comb_group: Option<Id>,
        /// External cells that may execute with this invoke.
        ref_cells: Vec<(Id, Id)>,
    },
    /// Invoke component with input/output assignments.
    StaticInvoke {
        /// Name of the component to be invoked.
        comp: Id,
        /// Input assignments
        inputs: Vec<(Id, Atom)>,
        /// Output assignments
        outputs: Vec<(Id, Atom)>,
        /// Attributes
        attributes: Attributes,
        /// External cells that may execute with this invoke.
        ref_cells: Vec<(Id, Id)>,
        /// Combinational group that may execute with this invoke.
        comb_group: Option<Id>,
        /// (optional) latency. Latency can be inferred if not given.
        latency: Option<NonZeroU64>,
    },
    /// Control statement that does nothing.
    Empty {
        /// Attributes
        attributes: Attributes,
    },
    /// Represents sequential composition of static control statements.
    StaticSeq {
        /// List of `Control` statements to run in sequence.
        /// If not all of these stmts are static, we should error out
        stmts: Vec<Control>,
        /// Attributes
        attributes: Attributes,
        /// Optional latency for the seq
        latency: Option<NonZeroU64>,
    },
    /// Represents parallel composition of static control statements.
    StaticPar {
        /// List of `Control` statements to run in sequence.
        /// If not all of these stmts are static, we should error out
        stmts: Vec<Control>,
        /// Attributes
        attributes: Attributes,
        /// Optional latency for the par
        latency: Option<NonZeroU64>,
    },
    /// Static if statement.
    StaticIf {
        /// Port that connects the conditional check.
        port: Port,

        /// Control for the true branch.
        tbranch: Box<Control>,

        /// Control for the true branch.
        fbranch: Box<Control>,

        /// Attributes
        attributes: Attributes,

        /// Optional latency; should be the longer of the two branches
        latency: Option<NonZeroU64>,
    },
    /// Static Repeat (essentially a bounded while loop w/o a condition)
    StaticRepeat {
        /// Control for the true branch.
        num_repeats: u64,

        /// Control for the true branch.
        body: Box<Control>,

        /// Attributes
        attributes: Attributes,
    },
}

impl Control {
    pub fn empty() -> Control {
        Control::Empty {
            attributes: Attributes::default(),
        }
    }

    pub fn get_attributes(&self) -> &Attributes {
        match self {
            Control::Seq { attributes, .. } => attributes,
            Control::Par { attributes, .. } => attributes,
            Control::If { attributes, .. } => attributes,
            Control::While { attributes, .. } => attributes,
            Control::Repeat { attributes, .. } => attributes,
            Control::Enable { attributes, .. } => attributes,
            Control::Invoke { attributes, .. } => attributes,
            Control::Empty { attributes, .. } => attributes,
            Control::StaticSeq { attributes, .. } => attributes,
            Control::StaticPar { attributes, .. } => attributes,
            Control::StaticIf { attributes, .. } => attributes,
            Control::StaticRepeat { attributes, .. } => attributes,
            Control::StaticInvoke { attributes, .. } => attributes,
        }
    }
}
