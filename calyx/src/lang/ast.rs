// use crate::errors::Error;
// use crate::lang::context::LibraryContext;
use std::collections::HashMap;

// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

// XXX(sam) Add location information to this type so that we can print
// them out nicely
/// Represents an identifier in a Futil program
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    id: String,
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
        Id { id: s.to_string() }
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id { id: s }
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}
/* =================== Impls for Id to make them easier to use ============== */

/// Top level AST statement. This contains a list of Component definitions.
#[derive(Clone, Debug, Hash)]
pub struct NamespaceDef {
    /// The path to libraries
    pub library: Option<ImportStatement>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
}

/// import statement
#[derive(Clone, Debug, Hash)]
pub struct ImportStatement {
    pub libraries: Vec<String>,
}

/// AST statement for defining components.
#[derive(Clone, Debug, Hash)]
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

// impl ComponentDef {
//     /// Given a Library Context, resolve all the primitive components
//     /// in `self` and return the signatures in a HashMap
//     pub fn resolve_primitives(
//         &self,
//         libctx: &LibraryContext,
//     ) -> Result<HashMap<Id, Signature>, Error> {
//         let mut map = HashMap::new();

//         for stmt in &self.cells {
//             if let Cell::Prim { data } = stmt {
//                 let sig = libctx
//                     .resolve(&data.instance.name, &data.instance.params)?;
//                 map.insert(data.name.clone(), sig);
//             }
//         }

//         Ok(map)
//     }
// }

/// The signature for a component. Contains a list
/// of input ports and a list of output ports.
#[derive(Clone, Debug, Hash)]
pub struct Signature {
    /// List of input ports.
    pub inputs: Vec<Portdef>,

    /// List of output ports.
    pub outputs: Vec<Portdef>,
}

impl Signature {
    pub fn has_input(&self, name: &str) -> bool {
        self.inputs.iter().any(|e| &e.name == name)
    }

    pub fn has_output(&self, name: &str) -> bool {
        self.outputs.iter().any(|e| &e.name == name)
    }
}

/// The definition of an input/output port.
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Portdef {
    /// The name of the port.
    pub name: Id,

    /// The width of the port.
    pub width: u64,
}

/// Helper to construct portdef from str and u64.
impl From<(&str, u64)> for Portdef {
    fn from((name, width): (&str, u64)) -> Self {
        Portdef {
            name: name.into(),
            width,
        }
    }
}

/// Statement that refers to a port on a subcomponent.
/// This is distinct from a `Portdef` which defines a port.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Compinst {
    /// Name of the subcomponent to instantiate.
    pub name: Id,

    /// List of parameters.
    pub params: Vec<u64>,
}

// ===================================
// AST for wire guard expressions
// ===================================

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Port(Port),
    Num(u64),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuardExpr {
    Eq(Atom, Atom),
    Neq(Atom, Atom),
    Gt(Atom, Atom),
    Lt(Atom, Atom),
    Geq(Atom, Atom),
    Leq(Atom, Atom),
    Atom(Atom),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Guard {
    pub exprs: Vec<GuardExpr>,
}

// ===================================
// Data definitions for Structure
// ===================================

/// Data for the `new` structure statement.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decl {
    /// Name of the variable being defined.
    pub name: Id,

    /// Name of the component being instantiated.
    pub component: Id,
}

/// Data for the `new-std` structure statement.
#[derive(Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Prim {
    /// Name of the variable being defined.
    pub name: Id,

    /// Data for instantiating the library component.
    pub instance: Compinst,
}

/// The Cell AST nodes.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Connection {
    Group(Group),
    Wire(Wire),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Group {
    pub name: Id,
    pub wires: Vec<Wire>,
}

/// Data for the `->` structure statement.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wire {
    /// Source of the wire.
    pub src: Vec<(Guard, Atom)>,

    /// Guarded destinations of the wire.
    pub dest: Port,
}

// ===================================
// Data definitions for Control Ast
// ===================================

/// Data for the `seq` control statement.
#[derive(Debug, Clone, Hash)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
}

/// Data for the `par` control statement.
#[derive(Debug, Clone, Hash)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
}

/// Data for the `if` control statement.
#[derive(Debug, Clone, Hash)]
pub struct If {
    /// Port that connects the conditional check.
    pub port: Port,

    /// Modules that need to be enabled to send signal on `port`.
    pub cond: Option<Id>,

    /// Control for the true branch.
    pub tbranch: Box<Control>,

    /// Control for the true branch.
    pub fbranch: Box<Control>,
}

/// Data for the `if` control statement.
#[derive(Debug, Clone, Hash)]
pub struct While {
    /// Port that connects the conditional check.
    pub port: Port,

    /// Modules that need to be enabled to send signal on `port`.
    pub cond: Option<Id>,

    /// Control for the loop body.
    pub body: Box<Control>,
}

/// Data for the `print` control statement.
#[derive(Debug, Clone, Hash)]
pub struct Print {
    /// Name of the port to print.
    pub var: Port,
}

/// Data for the `enable` control statement.
#[derive(Debug, Clone, Hash)]
pub struct Enable {
    /// List of components to run.
    pub comp: Id,
}

/// Data for the `empty` control statement.
#[derive(Debug, Clone, Hash)]
pub struct Empty {}

/// Control AST nodes.
#[derive(Debug, Clone, Hash)]
pub enum Control {
    /// Represents sequential composition of control statements.
    Seq { data: Seq },
    /// Represents parallel composition of control statements.
    Par { data: Par },
    /// Standard imperative if statement
    If { data: If },
    /// Standard imperative while statement
    While { data: While },
    /// Statement that prints out the value of a port during simulation.
    Print { data: Print },
    /// Runs the control for a list of subcomponents.
    Enable { data: Enable },
    /// Control statement that does nothing.
    Empty { data: Empty },
}

/// Methods for constructing control AST nodes.
#[allow(unused)]
impl Control {
    pub fn seq(stmts: Vec<Control>) -> Control {
        Control::Seq {
            data: Seq { stmts },
        }
    }

    pub fn par(stmts: Vec<Control>) -> Control {
        Control::Par {
            data: Par { stmts },
        }
    }

    pub fn c_if(
        port: Port,
        cond: Option<Id>,
        tbranch: Control,
        fbranch: Control,
    ) -> Control {
        Control::If {
            data: If {
                port,
                cond,
                tbranch: Box::new(tbranch),
                fbranch: Box::new(fbranch),
            },
        }
    }

    pub fn c_while(port: Port, cond: Option<Id>, body: Control) -> Control {
        Control::While {
            data: While {
                port,
                cond,
                body: Box::new(body),
            },
        }
    }

    pub fn print(var: Port) -> Control {
        Control::Print {
            data: Print { var },
        }
    }

    pub fn enable(comp: Id) -> Control {
        Control::Enable {
            data: Enable { comp },
        }
    }

    pub fn empty() -> Control {
        Control::Empty { data: Empty {} }
    }
}
