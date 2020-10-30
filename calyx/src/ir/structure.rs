//! Representation for structure (wires and cells) in a FuTIL program.
use super::{Guard, Id, RRC, WRC};
use std::collections::HashMap;
use std::rc::Rc;

/// Direction of a port on a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Input-Output "port". Should only be used by holes.
    Inout,
}

impl Direction {
    /// Return the direction opposite to the current direction
    pub fn reverse(&self) -> Self {
        match self {
            Direction::Input => Direction::Output,
            Direction::Output => Direction::Input,
            Direction::Inout => Direction::Inout,
        }
    }
}

/// Ports can come from Cells or Groups
#[derive(Debug, Clone)]
pub enum PortParent {
    Cell(WRC<Cell>),
    Group(WRC<Group>),
}

/// Represents a port on a cell.
#[derive(Debug, Clone)]
pub struct Port {
    /// Name of the port
    pub name: Id,
    /// Width of the port
    pub width: u64,
    /// Direction of the port
    pub direction: Direction,
    /// Weak pointer to this port's parent
    pub parent: PortParent,
}

impl Port {
    /// Checks if this port is a hole
    pub fn is_hole(&self) -> bool {
        matches!(&self.parent, PortParent::Group(_))
    }

    /// Checks if this port is a constant of value: `val`.
    pub fn is_constant(&self, val: u64) -> bool {
        if let PortParent::Cell(cell) = &self.parent {
            match cell.upgrade().unwrap().borrow().prototype {
                CellType::Constant { val: v, .. } => v == val,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Gets name of parent object.
    pub fn get_parent_name(&self) -> Id {
        match &self.parent {
            PortParent::Cell(cell) => cell
                .upgrade()
                .unwrap_or_else(|| panic!("No cell for port: {}", self.name))
                .borrow()
                .name
                .clone(),
            PortParent::Group(group) => group
                .upgrade()
                .unwrap_or_else(|| panic!("No group for hole: {}", self.name))
                .borrow()
                .name
                .clone(),
        }
    }
}

/// Alias for bindings
pub type Binding = Vec<(Id, u64)>;

/// The type for a Cell
#[derive(Debug)]
pub enum CellType {
    /// Cell constructed using a primitive definition
    Primitive {
        /// Name of the primitive cell used to instantiate this cell.
        name: Id,
        /// Bindings for the parameters. Uses Vec to retain the input order.
        param_binding: Binding,
    },
    /// Cell constructed using a FuTIL component
    Component {
        /// Name of the component used to instantiate this cell.
        name: Id,
    },
    /// This cell represents the current component
    ThisComponent,
    /// Cell representing a Constant
    Constant {
        /// Value of this constant
        val: u64,
        /// Width of this constant
        width: u64,
    },
}

/// Represents an instantiated cell.
#[derive(Debug)]
pub struct Cell {
    /// Name of this cell.
    pub name: Id,
    /// Ports on this cell
    pub ports: Vec<RRC<Port>>,
    /// Underlying type for this cell
    pub prototype: CellType,
}

impl Cell {
    /// Get a reference to the named port if it exists.
    pub fn find<S>(&self, name: S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.ports
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Get a reference to the named port and throw an error if it doesn't
    /// exist.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.find(&name).unwrap_or_else(|| {
            panic!(
                "Port `{}' not found on group `{}'",
                name.to_string(),
                self.name.to_string()
            )
        })
    }

    /// Returns the name of the component that is this cells type.
    pub fn type_name(&self) -> Option<&Id> {
        match &self.prototype {
            CellType::Primitive { name, .. } | CellType::Component { name } => {
                Some(name)
            }
            CellType::ThisComponent => Some(&self.name),
            CellType::Constant { .. } => None,
        }
    }

    /// Return the canonical name for the cell generated to represent this
    /// (val, width) constant.
    pub(super) fn constant_name(val: u64, width: u64) -> Id {
        format!("_{}_{}", val, width).into()
    }
}

/// Represents a guarded assignment in the program
#[derive(Debug)]
pub struct Assignment {
    /// The destination for the assignment.
    pub dst: RRC<Port>,

    /// The source for the assignment.
    pub src: RRC<Port>,

    /// The guard for this assignment.
    pub guard: Guard,
}

/// A Group of assignments that perform a logical action.
#[derive(Debug)]
pub struct Group {
    /// Name of this group
    pub name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment>,

    /// Holes for this group
    pub holes: Vec<RRC<Port>>,

    /// Attributes for this group.
    pub attributes: HashMap<String, u64>,
}

impl Group {
    /// Get a reference to the named hole if it exists.
    pub fn find<S>(&self, name: &S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.holes
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Get a reference to the named hole or panic.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.find(&name).unwrap_or_else(|| {
            panic!(
                "Hole `{}' not found on group `{}'",
                name.to_string(),
                self.name.to_string()
            )
        })
    }
}
