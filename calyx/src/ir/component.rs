use super::{Assignment, Cell, Control, Group, RRC, Id};
use std::rc::Rc;

/// In memory representation of a Component.
#[derive(Debug)]
pub struct Component {
    /// Name of the component.
    pub name: Id,
    /// The input/output signature of this component.
    pub signature: RRC<Cell>,
    /// The cells instantiated for this component.
    pub cells: Vec<RRC<Cell>>,
    /// Groups of assignment wires.
    pub groups: Vec<RRC<Group>>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Vec<Assignment>,
    /// The control program for this component.
    pub control: RRC<Control>,
}

/// Builder methods for extracting and construction IR nodes.
/// The naming scheme for methods is consistent:
/// - find_<construct>: Returns a reference to the construct with the given
///   name.
impl Component {
    /// Return a reference to the group with `name` if present.
    pub fn find_group(&self, name: &Id) -> Option<RRC<Group>> {
        self.groups
            .iter()
            .find(|&g| g.borrow().name == *name)
            .map(|r| Rc::clone(r))
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell(&self, name: &Id) -> Option<RRC<Cell>> {
        self.cells
            .iter()
            .find(|&g| g.borrow().name == *name)
            .map(|r| Rc::clone(r))
    }
}
