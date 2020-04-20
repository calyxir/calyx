use crate::errors::Error;
use crate::lang::context::LibraryContext;
use sexpy::Sexpy;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Abstract Syntax Tree for Futil. See link below for the grammar
// https://github.com/cucapra/futil/blob/master/grammar.md

// XXX(sam) Add location information to this type so that we can print
// them out nicely
/// Represents an identifier in a Futil program
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(nohead, nosurround)]
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

/// Parses a pathbuf into a NamespaceDef
pub fn parse_file(file: &PathBuf) -> Result<NamespaceDef, Error> {
    let content = &fs::read(file)?;
    let string_content = std::str::from_utf8(content)?;
    match NamespaceDef::parse(string_content) {
        Ok(ns) => Ok(ns),
        Err(msg) => Err(Error::ParseError(msg)),
    }
}

/// Top level AST statement. This contains a list of Component definitions.
#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(head = "define/namespace")]
pub struct NamespaceDef {
    /// Name of the namespace.
    pub name: Id,
    /// The path to libraries
    pub library: Option<ImportStatement>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
}

/// import statement
#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(head = "import")]
pub struct ImportStatement {
    pub libraries: Vec<String>,
}

/// AST statement for defining components.
#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(head = "define/component")]
pub struct ComponentDef {
    /// Name of the component.
    pub name: Id,

    /// Defines input and output ports.
    pub signature: Signature,

    /// List of structure statements for this component.
    #[sexpy(surround)]
    pub structure: Vec<Structure>,

    /// Single control statement for this component.
    pub control: Control,
}

impl ComponentDef {
    /// Given a Library Context, resolve all the primitive components
    /// in `self` and return the signatures in a HashMap
    pub fn resolve_primitives(
        &self,
        libctx: &LibraryContext,
    ) -> Result<HashMap<Id, Signature>, Error> {
        let mut map = HashMap::new();

        for stmt in &self.structure {
            if let Structure::Std { data } = stmt {
                let sig = libctx
                    .resolve(&data.instance.name, &data.instance.params)?;
                map.insert(data.name.clone(), sig);
            }
        }

        Ok(map)
    }
}

/// The signature for a component. Contains a list
/// of input ports and a list of output ports.
#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(nohead, nosurround)]
pub struct Signature {
    /// List of input ports.
    #[sexpy(surround)]
    pub inputs: Vec<Portdef>,

    /// List of output ports.
    #[sexpy(surround)]
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
#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(head = "port")]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Sexpy, PartialOrd, Ord)]
#[sexpy(head = "@")]
pub enum Port {
    /// Refers to the port named `port` on the subcomponent
    /// `component`.
    Comp { component: Id, port: Id },

    /// Refers to the port named `port` on the component
    /// currently being defined.
    #[sexpy(head = "this")]
    This { port: Id },
}

impl Port {
    /// Returns the name of the port being referenced.
    ///  - `(@ comp A)` returns `A`
    ///  - `(@ this B)` returns `B`
    pub fn port_name(&self) -> &Id {
        match self {
            Port::Comp { port, .. } => port,
            Port::This { port } => port,
        }
    }
}

/// Instantiates a subcomponent named `name` with
/// paramters `params`.
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(nohead)]
pub struct Compinst {
    /// Name of the subcomponent to instantiate.
    pub name: Id,

    /// List of parameters.
    pub params: Vec<u64>,
}

// ===================================
// Data definitions for Structure
// ===================================

/// Data for the `new` structure statement.
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(head = "new", nosurround)]
pub struct Decl {
    /// Name of the variable being defined.
    pub name: Id,

    /// Name of the component being instantiated.
    pub component: Id,
}

/// Data for the `new-std` structure statement.
#[derive(Clone, Debug, Hash, Sexpy, Eq, PartialEq, PartialOrd, Ord)]
#[sexpy(head = "new-std", nosurround)]
pub struct Std {
    /// Name of the variable being defined.
    pub name: Id,

    /// Data for instantiating the library component.
    pub instance: Compinst,
}

/// Data for the `->` structure statement.
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(head = "->", nosurround)]
pub struct Wire {
    /// Source of the wire.
    pub src: Port,

    /// Destination of the wire.
    pub dest: Port,
}

/// Group definition
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(head = "group", nosurround)]
pub struct Group {
    /// Name of the group.
    pub name: Id,
    /// Subcomponents included in the group.
    #[sexpy(surround)]
    pub comps: Vec<Id>,
}

/// The Structure AST nodes.
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq, PartialOrd, Ord)]
#[sexpy(nohead)]
pub enum Structure {
    /// Node for instantiating user-defined components.
    Decl { data: Decl },
    /// Node for instantiating primitive components.
    Std { data: Std },
    /// Node for connecting ports on different components.
    Wire { data: Wire },
    /// Node for group definitions.
    Group { data: Group },
}

/// Methods for constructing the structure AST nodes.
#[allow(unused)]
impl Structure {
    /// Constructs `Structure::Decl` with `name` and `component`
    /// as arguments.
    pub fn decl(name: Id, component: Id) -> Structure {
        Structure::Decl {
            data: Decl { name, component },
        }
    }

    /// Constructs `Structure::Std` with `name` and `instance`
    /// as arguments.
    pub fn std(name: Id, instance: Compinst) -> Structure {
        Structure::Std {
            data: Std { name, instance },
        }
    }

    /// Constructs `Structure::Wire` with `src` and `dest`
    /// as arguments.
    pub fn wire(src: Port, dest: Port) -> Structure {
        Structure::Wire {
            data: Wire { src, dest },
        }
    }
}

// ===================================
// Data definitions for Control Ast
// ===================================

/// Data for the `seq` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Seq {
    /// List of `Control` statements to run in sequence.
    pub stmts: Vec<Control>,
}

/// Data for the `par` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Par {
    /// List of `Control` statements to run in parallel.
    pub stmts: Vec<Control>,
}

/// Data for the `if` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct If {
    /// Port that connects the conditional check.
    pub port: Port,

    /// Modules that need to be enabled to send signal on `port`.
    #[sexpy(surround)]
    pub cond: Vec<Id>,

    /// Control for the true branch.
    pub tbranch: Box<Control>,

    /// Control for the true branch.
    pub fbranch: Box<Control>,
}

/// Data for the `if` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct While {
    /// Port that connects the conditional check.
    pub port: Port,

    /// Modules that need to be enabled to send signal on `port`.
    #[sexpy(surround)]
    pub cond: Vec<Id>,

    /// Control for the loop body.
    pub body: Box<Control>,
}

/// Data for the `print` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Print {
    /// Name of the port to print.
    pub var: Port,
}

/// Data for the `enable` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Enable {
    /// Group Id to run
    pub group: Id,
}

/// Data for the `empty` control statement.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Empty {}

/// Control AST nodes.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nohead)]
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
        stmts: Vec<Id>,
        tbranch: Control,
        fbranch: Control,
    ) -> Control {
        Control::If {
            data: If {
                port,
                cond: stmts,
                tbranch: Box::new(tbranch),
                fbranch: Box::new(fbranch),
            },
        }
    }

    pub fn c_while(port: Port, stmts: Vec<Id>, body: Control) -> Control {
        Control::While {
            data: While {
                port,
                cond: stmts,
                body: Box::new(body),
            },
        }
    }

    pub fn print(var: Port) -> Control {
        Control::Print {
            data: Print { var },
        }
    }

    pub fn enable(group: Id) -> Control {
        Control::Enable {
            data: Enable { group },
        }
    }

    pub fn empty() -> Control {
        Control::Empty { data: Empty {} }
    }
}
