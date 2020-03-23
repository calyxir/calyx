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
#[derive(Clone, Debug, Hash, Sexpy, PartialEq, Eq)]
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

#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(head = "define/namespace")]
pub struct NamespaceDef {
    pub name: Id,
    pub components: Vec<ComponentDef>,
}

#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(head = "define/component")]
pub struct ComponentDef {
    pub name: Id,
    pub signature: Signature,
    #[sexpy(surround)]
    pub structure: Vec<Structure>,
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

#[derive(Clone, Debug, Hash, Sexpy)]
#[sexpy(nohead, nosurround)]
pub struct Signature {
    #[sexpy(surround)]
    pub inputs: Vec<Portdef>,
    #[sexpy(surround)]
    pub outputs: Vec<Portdef>,
}

impl Signature {
    pub fn has_input(&self, name: &str) -> bool {
        self.inputs.iter().any(|e| &e.name == name)
    }
    // pub fn new(inputs: &[(&str, u64)], outputs: &[(&str, u64)]) -> Self {
    //     Signature {
    //         inputs: inputs.iter().map(|x| x.into()).collect(),
    //         outputs: outputs.iter().map(|x| x.into()).collect(),
    //     }
    // }
}

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(head = "port")]
pub struct Portdef {
    pub name: Id,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Sexpy)]
#[sexpy(head = "@")]
pub enum Port {
    Comp {
        component: Id,
        port: Id,
    },
    #[sexpy(head = "this")]
    This {
        port: Id,
    },
}

impl Port {
    pub fn port_name(&self) -> &Id {
        match self {
            Port::Comp { port, .. } => port,
            Port::This { port } => port,
        }
    }
}

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(nohead)]
pub struct Compinst {
    pub name: Id,
    pub params: Vec<u64>,
}

// ===================================
// Data definitions for Structure
// ===================================

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(head = "new", nosurround)]
pub struct Decl {
    pub name: Id,
    pub component: Id,
}

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(head = "new-std", nosurround)]
pub struct Std {
    pub name: Id,
    pub instance: Compinst,
}

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(head = "->", nosurround)]
pub struct Wire {
    pub src: Port,
    pub dest: Port,
}

#[derive(Clone, Debug, Hash, Sexpy, PartialEq)]
#[sexpy(nohead)]
pub enum Structure {
    Decl { data: Decl },
    Std { data: Std },
    Wire { data: Wire },
}

#[allow(unused)]
impl Structure {
    pub fn decl(name: Id, component: Id) -> Structure {
        Structure::Decl {
            data: Decl { name, component },
        }
    }

    pub fn std(name: Id, instance: Compinst) -> Structure {
        Structure::Std {
            data: Std { name, instance },
        }
    }

    pub fn wire(src: Port, dest: Port) -> Structure {
        Structure::Wire {
            data: Wire { src, dest },
        }
    }
}

// ===================================
// Data definitions for Control Ast
// ===================================

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Seq {
    pub stmts: Vec<Control>,
}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Par {
    pub stmts: Vec<Control>,
}

// If control node in the AST.
#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct If {
    // Port that connects the conditional check.
    pub port: Port,

    #[sexpy(surround)]
    // Modules that need to be enabled to send signal on `port`.
    pub cond: Vec<Id>,

    // Control for the true branch.
    pub tbranch: Box<Control>,

    // Control for the true branch.
    pub fbranch: Box<Control>,
}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct While {
    pub port: Port,
    #[sexpy(surround)]
    pub cond: Vec<Id>,
    pub body: Box<Control>,
}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Print {
    pub var: Id,
}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Enable {
    pub comps: Vec<Id>,
}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nosurround)]
pub struct Empty {}

#[derive(Debug, Clone, Hash, Sexpy)]
#[sexpy(nohead)]
pub enum Control {
    Seq { data: Seq },
    Par { data: Par },
    If { data: If },
    While { data: While },
    Print { data: Print },
    Enable { data: Enable },
    Empty { data: Empty },
}

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

    pub fn print(var: Id) -> Control {
        Control::Print {
            data: Print { var },
        }
    }

    pub fn enable(comps: Vec<Id>) -> Control {
        Control::Enable {
            data: Enable { comps },
        }
    }

    pub fn empty() -> Control {
        Control::Empty { data: Empty {} }
    }
}
