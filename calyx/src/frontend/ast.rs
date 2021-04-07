/// Abstract Syntax Tree for Futil
use super::parser;
use crate::errors::{Error, FutilResult, Span};
use crate::ir;
use atty::Stream;
use std::io::stdin;
use std::path::{Path, PathBuf};

/// Represents the parsed AST of a complete program. Contains all the components
/// and primitives that were encountered during the parsing the program.
///
/// # Example
/// When parsing a file `foo.futil`:
/// ```
/// import "core.futil";
///
/// component main() -> () { ... }
/// ```
/// `main` is added to the current namespace and `core.futil` is added to
/// the parsing queue. Next, `core.futil` is parsed:
/// ```
/// extern "core.sv" {
///     primitive std_add[width](left: width, right: width) -> (out: width);
/// }
/// ```
/// The primitive `std_add` is added to the namespace and `"core.sv"` is
/// added to the set of paths that need to be "linked" in the backend
/// generation.
///
/// Since `core.futil` does not `import` any file, the parsing process is
/// completed.
#[derive(Debug)]
pub struct NamespaceDef {
    /// Path to extern files.
    pub imports: Vec<String>,
    /// List of component definitions.
    pub components: Vec<ComponentDef>,
    /// Extern statements and any primitive declarations in them.
    pub externs: Vec<(String, Vec<ir::Primitive>)>,
}

impl NamespaceDef {
    /// Parse the program and all of its transitive dependencies to build
    /// a whole program context.
    pub fn new(file: &Option<PathBuf>, lib_path: &Path) -> FutilResult<Self> {
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

/// AST statement for defining components.
#[derive(Debug)]
pub struct ComponentDef {
    /// Name of the component.
    pub name: ir::Id,
    /// Defines input and output ports along with their attributes.
    pub signature: Vec<ir::PortDef>,
    /// List of instantiated sub-components
    pub cells: Vec<Cell>,
    /// List of groups
    pub groups: Vec<Group>,
    /// List of continuous assignments
    pub continuous_assignments: Vec<Wire>,
    /// Single control statement for this component.
    pub control: Control,
    /// Attributes attached to this component
    pub attributes: ir::Attributes,
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
    // Logical operations
    And(Box<GuardExpr>, Box<GuardExpr>),
    Or(Box<GuardExpr>, Box<GuardExpr>),
    Not(Box<GuardExpr>),
    // Comparison operations
    Eq(Atom, Atom),
    Neq(Atom, Atom),
    Gt(Atom, Atom),
    Lt(Atom, Atom),
    Geq(Atom, Atom),
    Leq(Atom, Atom),
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

/// Prototype of the cell definition
#[derive(Debug)]
pub struct Proto {
    /// Name of the primitive.
    pub name: ir::Id,
    /// Parameter binding for primitives
    pub params: Vec<u64>,
}

/// The Cell AST nodes.
#[derive(Debug)]
pub struct Cell {
    /// Name of the cell.
    pub name: ir::Id,
    /// Name of the prototype this cell was built from.
    pub prototype: Proto,
    /// Attributes attached to this cell definition
    pub attributes: ir::Attributes,
}

/// Methods for constructing the structure AST nodes.
impl Cell {
    /// Constructs a primitive cell instantiation.
    pub fn from(
        name: ir::Id,
        proto: ir::Id,
        params: Vec<u64>,
        attributes: ir::Attributes,
    ) -> Cell {
        Cell {
            name,
            prototype: Proto {
                name: proto,
                params,
            },
            attributes,
        }
    }
}

#[derive(Debug)]
pub struct Group {
    pub name: ir::Id,
    pub wires: Vec<Wire>,
    pub attributes: ir::Attributes,
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
        /// Attributes
        attributes: ir::Attributes,
    },
    /// Represents parallel composition of control statements.
    Par {
        /// List of `Control` statements to run in sequence.
        stmts: Vec<Control>,
        /// Attributes
        attributes: ir::Attributes,
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

        /// Attributes
        attributes: ir::Attributes,
    },
    /// Standard imperative while statement
    While {
        /// Port that connects the conditional check.
        port: Port,

        /// Modules that need to be enabled to send signal on `port`.
        cond: ir::Id,

        /// Control for the loop body.
        body: Box<Control>,

        /// Attributes
        attributes: ir::Attributes,
    },
    /// Runs the control for a list of subcomponents.
    Enable {
        /// Group to be enabled
        comp: ir::Id,
        /// Attributes
        attributes: ir::Attributes,
    },
    /// Invoke component with input/output assignments.
    Invoke {
        /// Name of the component to be invoked.
        comp: ir::Id,
        /// Input assignments
        inputs: Vec<(ir::Id, Port)>,
        /// Output assignments
        outputs: Vec<(ir::Id, Port)>,
        /// Attributes
        attributes: ir::Attributes,
    },
    /// Control statement that does nothing.
    Empty {},
}
