/// Abstract Syntax Tree for Futil
use crate::errors::Span;
use crate::ir;
use std::collections::HashMap;

/// Top level AST statement. This contains a list of Component definitions.
#[derive(Debug)]
pub struct NamespaceDef {
    /// The path to libraries
    pub libraries: Vec<String>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
}

/// AST statement for defining components.
#[derive(Debug)]
pub struct ComponentDef {
    /// Name of the component.
    pub name: ir::Id,

    /// Defines input and output ports.
    pub signature: Signature,

    /// List of instantiated sub-components
    pub cells: Vec<Cell>,

    /// List of groups
    pub groups: Vec<Group>,

    /// List of continuous assignments
    pub continuous_assignments: Vec<Wire>,

    /// Single control statement for this component.
    pub control: Control,
}

/// The signature for a component. Contains a list
/// of input ports and a list of output ports.
#[derive(Clone, Debug)]
pub struct Signature {
    /// List of input ports.
    pub inputs: Vec<Portdef>,

    /// List of output ports.
    pub outputs: Vec<Portdef>,
}

/// The definition of an input/output port.
#[derive(Clone, Debug)]
pub struct Portdef {
    /// The name of the port.
    pub name: ir::Id,

    /// The width of the port.
    pub width: u64,
}

/// Statement that refers to a port on a subcomponent.
/// This is distinct from a `Portdef` which defines a port.
#[derive(Debug)]
pub enum Port {
    /// Refers to the port named `port` on the subcomponent
    /// `component`.
    Comp { component: ir::Id, port: ir::Id },

    /// Refers to the port named `port` on the component
    /// currently being defined.
    This { port: ir::Id },

    /// `group[name]` parses into `Hole { group, name }`
    /// and is a hole named `name` on group `group`
    Hole { group: ir::Id, name: ir::Id },
}

impl Port {
    /// Returns the name of the port being referenced.
    ///  - `(@ comp A)` returns `A`
    ///  - `(@ this B)` returns `B`
    pub fn port_name(&self) -> &ir::Id {
        match self {
            Port::Comp { port, .. } => port,
            Port::This { port } => port,
            Port::Hole { name, .. } => name,
        }
    }
}

/// Instantiates a subcomponent named `name` with
/// paramters `params`.
#[derive(Debug)]
pub struct Compinst {
    /// Name of the subcomponent to instantiate.
    pub name: ir::Id,

    /// List of parameters.
    pub params: Vec<u64>,
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
    pub span: Option<Span>,
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
    // TODO(rachit): Go back to the simpler, two children AST representation.
    // Use the IR to merge And nodes.
    And(Vec<GuardExpr>),
    Or(Vec<GuardExpr>),
    Eq(Box<GuardExpr>, Box<GuardExpr>),
    Neq(Box<GuardExpr>, Box<GuardExpr>),
    Gt(Box<GuardExpr>, Box<GuardExpr>),
    Lt(Box<GuardExpr>, Box<GuardExpr>),
    Geq(Box<GuardExpr>, Box<GuardExpr>),
    Leq(Box<GuardExpr>, Box<GuardExpr>),
    Not(Box<GuardExpr>),
    Atom(Atom),
}

/// A guard is a conditions in `guard_conj` which guard the value
/// represented by `expr`.
#[derive(Debug)]
pub struct Guard {
    pub guard: Option<GuardExpr>,
    pub expr: Atom,
}

// ===================================
// Data definitions for Structure
// ===================================

/// Data for the `new` structure statement.
#[derive(Debug)]
pub struct Decl {
    /// Name of the variable being defined.
    pub name: ir::Id,

    /// Name of the component being instantiated.
    pub component: ir::Id,
}

/// Data for the `new-std` structure statement.
#[derive(Debug)]
pub struct Prim {
    /// Name of the variable being defined.
    pub name: ir::Id,

    /// Data for instantiating the library component.
    pub instance: Compinst,
}

/// The Cell AST nodes.
#[derive(Debug)]
pub enum Cell {
    /// Node for instantiating user-defined components.
    Decl { data: Decl },
    /// Node for instantiating primitive components.
    Prim { data: Prim },
}

/// Methods for constructing the structure AST nodes.
impl Cell {
    /// Constructs `Structure::Decl` with `name` and `component`
    /// as arguments.
    pub fn decl(name: ir::Id, component: ir::Id) -> Cell {
        Cell::Decl {
            data: Decl { name, component },
        }
    }

    /// Constructs `Structure::Std` with `name` and `instance`
    /// as arguments.
    pub fn prim(var: ir::Id, prim_name: ir::Id, params: Vec<u64>) -> Cell {
        Cell::Prim {
            data: Prim {
                name: var,
                instance: Compinst {
                    name: prim_name,
                    params,
                },
            },
        }
    }
}

#[derive(Debug)]
pub struct Group {
    pub name: ir::Id,
    pub wires: Vec<Wire>,
    pub attributes: HashMap<String, u64>,
}

/// Data for the `->` structure statement.
#[derive(Debug)]
pub struct Wire {
    /// Source of the wire.
    pub src: Guard,

    /// Guarded destinations of the wire.
    pub dest: Port,
}

/// Control AST nodes.
#[derive(Debug)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq {
        /// List of `Control` statements to run in sequence.
        stmts: Vec<Control>,
    },
    /// Represents parallel composition of control statements.
    Par {
        /// List of `Control` statements to run in sequence.
        stmts: Vec<Control>,
    },
    /// Standard imperative if statement
    If {
        /// Port that connects the conditional check.
        port: Port,

        /// Modules that need to be enabled to send signal on `port`.
        cond: ir::Id,

        /// Control for the true branch.
        tbranch: Box<Control>,

        /// Control for the true branch.
        fbranch: Box<Control>,
    },
    /// Standard imperative while statement
    While {
        /// Port that connects the conditional check.
        port: Port,

        /// Modules that need to be enabled to send signal on `port`.
        cond: ir::Id,

        /// Control for the loop body.
        body: Box<Control>,
    },
    /// Runs the control for a list of subcomponents.
    Enable {
        /// Group to be enabled
        comp: ir::Id,
    },
    /// Control statement that does nothing.
    Empty {},
}
