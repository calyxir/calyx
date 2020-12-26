/// Abstract Syntax Tree for Futil
use super::parser;
use crate::errors::{Error, FutilResult, Span};
use crate::ir;
use linked_hash_map::LinkedHashMap;
use atty::Stream;
use std::collections::HashMap;
use std::io::stdin;
use std::path::{Path, PathBuf};

/// Top level AST statement. This contains a list of Component definitions.
#[derive(Debug)]
pub struct NamespaceDef {
    /// Path to imported files.
    pub imports: Vec<String>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
    /// Extern statements and any primitive declarations in them.
    pub externs: Vec<(String, Vec<Primitive>)>,
}

impl NamespaceDef {
    /// Parse the program and all of its transitive dependencies to build
    /// a whole program context.
    pub fn new(
        file: &Option<PathBuf>,
        lib_path: &PathBuf,
    ) -> FutilResult<Self> {
        let mut namespace = match file {
            Some(file) => parser::FutilParser::parse_file(&file),
            None => {
                if atty::isnt(Stream::Stdin) {
                    parser::FutilParser::parse(stdin())
                } else {
                    Err(Error::InvalidFile(
                        "No file provided and terminal not a TTY".to_string(),
                    ))
                }
            }
        }?;

        namespace.externs.iter_mut().for_each(|(path, _)| {
            *path = lib_path.join(path.clone()).to_string_lossy().to_string();
        });

        // Parse all transitive dependencies
        let mut deps: Vec<PathBuf> = namespace
            .imports
            .clone()
            .into_iter()
            .map(|f| lib_path.join(f))
            .collect();

        while let Some(path) = deps.pop() {
            let mut ns = parser::FutilParser::parse_file(&path)?;
            namespace.components.append(&mut ns.components);

            let parent = match path.parent() {
                Some(a) => a,
                None => Path::new("."),
            };

            namespace.externs.append(
                &mut ns
                    .externs
                    .into_iter()
                    .map(|(path, exts)| {
                        (parent.join(path).to_string_lossy().to_string(), exts)
                    })
                    .collect(),
            );

            // All imports are relative to the file being currently parsed.
            deps.append(
                &mut ns.imports.into_iter().map(|f| parent.join(f)).collect(),
            );
        }

        Ok(namespace)
    }
}

/// Representation of a Primitive.
#[derive(Clone, Debug)]
pub struct Primitive {
    /// Name of this primitive.
    pub name: ir::Id,
    /// Paramters for this primitive.
    pub params: Vec<ir::Id>,
    /// The input/output signature for this primitive.
    pub signature: Vec<PortDef>,
    /// Key-value attributes for this primitive.
    pub attributes: HashMap<String, u64>,
}

impl Primitive {
    /// Retuns the bindings for all the paramters, the input ports and the
    /// output ports.
    #[allow(clippy::type_complexity)]
    pub fn resolve(
        &self,
        parameters: &[u64],
    ) -> FutilResult<(Vec<(ir::Id, u64)>, Vec<(ir::Id, u64, ir::Direction)>)>
    {
        let bindings = self
            .params
            .iter()
            .cloned()
            .zip(parameters.iter().cloned())
            .collect::<LinkedHashMap<ir::Id, u64>>();

        let ports = self
            .signature
            .iter()
            .cloned()
            .map(|pd| pd.resolve(&bindings).map(|(n, w)| (n, w, pd.direction)))
            .collect::<Result<_, _>>()?;

        Ok((bindings.into_iter().collect(), ports))
    }
}

/// A parameter port definition.
#[derive(Clone, Debug)]
pub struct PortDef {
    pub name: ir::Id,
    pub width: Width,
    pub direction: ir::Direction,
}

impl From<(ir::Id, u64, ir::Direction)> for PortDef {
    fn from(port: (ir::Id, u64, ir::Direction)) -> Self {
        PortDef {
            name: port.0,
            width: Width::Const { value: port.1 },
            direction: port.2,
        }
    }
}

/// Represents an abstract width of a primitive signature.
#[derive(Clone, Debug)]
pub enum Width {
    /// The width is a constant.
    Const { value: u64 },
    /// The width is a parameter.
    Param { value: ir::Id },
}

impl PortDef {
    pub fn resolve(
        &self,
        binding: &LinkedHashMap<ir::Id, u64>,
    ) -> FutilResult<(ir::Id, u64)> {
        match &self.width {
            Width::Const { value } => Ok((self.name.clone(), *value)),
            Width::Param { value } => match binding.get(&value) {
                Some(width) => Ok((self.name.clone(), *width)),
                None => Err(Error::SignatureResolutionFailed(
                    self.name.clone(),
                    value.clone(),
                )),
            },
        }
    }
}

/// AST statement for defining components.
#[derive(Debug)]
pub struct ComponentDef {
    /// Name of the component.
    pub name: ir::Id,
    /// Defines input and output ports.
    pub signature: Vec<PortDef>,
    /// List of instantiated sub-components
    pub cells: Vec<Cell>,
    /// List of groups
    pub groups: Vec<Group>,
    /// List of continuous assignments
    pub continuous_assignments: Vec<Wire>,
    /// Single control statement for this component.
    pub control: Control,
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
    And(Box<GuardExpr>, Box<GuardExpr>),
    Or(Box<GuardExpr>, Box<GuardExpr>),
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

/// The Cell AST nodes.
#[derive(Debug)]
pub enum Cell {
    /// Node for instantiating user-defined components.
    Decl { name: ir::Id, component: ir::Id },
    /// Node for instantiating primitive components.
    Prim {
        name: ir::Id,
        prim: ir::Id,
        params: Vec<u64>,
    },
}

/// Methods for constructing the structure AST nodes.
impl Cell {
    /// Constructs `Structure::Std` with `name` and `instance`
    /// as arguments.
    pub fn prim(var: ir::Id, prim_name: ir::Id, params: Vec<u64>) -> Cell {
        Cell::Prim {
            name: var,
            prim: prim_name,
            params,
        }
    }

    /// Return the name of the cell.
    pub fn name(&self) -> &ir::Id {
        match self {
            Self::Decl { name, .. } => name,
            Self::Prim { name, .. } => name,
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
    /// Invoke component with input/output assignments.
    Invoke {
        /// Name of the component to be invoked.
        comp: ir::Id,
        /// Input assignments
        inputs: Vec<(ir::Id, Port)>,
        /// Output assignments
        outputs: Vec<(ir::Id, Port)>,
    },
    /// Control statement that does nothing.
    Empty {},
}
