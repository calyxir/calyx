use super::{Guard, WRC, RRC};
use crate::lang::ast::Id;
use std::collections::HashMap;

/// Direction of a port on a cell.
pub enum Direction {
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Input-Output "port". Should only be used by holes.
    Inout,
}

/// Ports can come from Cells or Groups
pub enum PortParent {
    Cell(WRC<Cell>),
    Group(WRC<Group>),
}

/// Represents a port on a cell.
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

/// The type for a Cell
pub enum CellType {
    /// Cell constructed using a primitive definition
    Primitive,
    /// Cell constructed using a FuTIL component
    Component,
    /// This cell represents the current component
    ThisComponent,
    /// Cell representing a Constant
    Constant,
}

/// Represents an instantiated cell.
pub struct Cell {
    /// Name of this cell.
    pub name: Id,
    /// Ports on this cell
    pub ports: Vec<RRC<Port>>,
    /// Underlying type for this cell
    pub prototype: CellType,
}

/// Represents a guarded assignment in the program
pub struct Assignment {
    /// The destination for the assignment.
    pub dst: RRC<Port>,

    /// The source for the assignment.
    pub src: RRC<Port>,

    /// The guard for this assignment.
    pub guard: Option<Guard>,
}

/// A Group of assignments that perform a logical action.
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


impl Cell {
    /// Return the canonical name for the cell generated to represent this
    /// (val, width) constant.
    pub(super) fn constant_name(val: u64, width: u64) -> Id {
        format!("_{}_{}", val, width).into()
    }
}
