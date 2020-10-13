// Abstract Syntax Tree for Futil
use crate::errors::Span;
use derivative::Derivative;
use std::collections::HashMap;

/// Represents an identifier in a Futil program
#[derive(Derivative, Clone, PartialOrd, Ord)]
#[derivative(Hash, Eq)]
pub struct Id {
    pub id: String,
    #[derivative(Hash = "ignore")]
    span: Option<Span>,
}

impl Id {
    pub fn new<S: ToString>(id: S, span: Option<Span>) -> Self {
        Self {
            id: id.to_string(),
            span,
        }
    }

    pub fn fmt_err(&self, err_msg: &str) -> String {
        match &self.span {
            Some(span) => span.format(err_msg),
            None => err_msg.to_string(),
        }
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Id").field("id", &self.id).finish()
    }
}

/* =================== Impls for Id to make them easier to use ============== */

impl ToString for Id {
    fn to_string(&self) -> String {
        self.id.clone()
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id {
            id: s.to_string(),
            span: None,
        }
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id { id: s, span: None }
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}

impl<S: AsRef<str>> PartialEq<S> for Id {
    fn eq(&self, other: &S) -> bool {
        self.id == other.as_ref()
    }
}

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
    pub name: Id,

    /// Defines input and output ports.
    pub signature: Signature,

    /// List of instantiated sub-components
    pub cells: Vec<Cell>,

    /// List of wires
    pub connections: Vec<Connection>,

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
    pub name: Id,

    /// The width of the port.
    pub width: u64,
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

impl Port {
    /// Returns the name of the port being referenced.
    ///  - `(@ comp A)` returns `A`
    ///  - `(@ this B)` returns `B`
    pub fn port_name(&self) -> &Id {
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
    pub name: Id,

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
    pub name: Id,

    /// Name of the component being instantiated.
    pub component: Id,
}

/// Data for the `new-std` structure statement.
#[derive(Debug)]
pub struct Prim {
    /// Name of the variable being defined.
    pub name: Id,

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
    pub fn decl(name: Id, component: Id) -> Cell {
        Cell::Decl {
            data: Decl { name, component },
        }
    }

    /// Constructs `Structure::Std` with `name` and `instance`
    /// as arguments.
    pub fn prim(var: Id, prim_name: Id, params: Vec<u64>) -> Cell {
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

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Connection {
    Group(Group),
    Wire(Wire),
}

#[derive(Debug)]
pub struct Group {
    pub name: Id,
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
        cond: Id,

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
        cond: Id,

        /// Control for the loop body.
        body: Box<Control>,
    },
    /// Runs the control for a list of subcomponents.
    Enable {
        /// Group to be enabled
        comp: Id,
    },
    /// Control statement that does nothing.
    Empty {},
}
