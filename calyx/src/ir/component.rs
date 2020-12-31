use super::{
    Assignment, Builder, Cell, CellType, Control, Direction, Group, Id, RRC,
};
use crate::utils;
use std::cell::RefCell;
use std::rc::Rc;

/// The default name of the signature cell in a component.
/// In general, this should not be used by anything.
const THIS_ID: &str = "_this";

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

    ///// Internal structures
    /// Namegenerator that contains the names currently defined in this
    /// component (cell and group names).
    namegen: utils::NameGenerator,
}

/// Builder methods for extracting and construction IR nodes.
/// The naming scheme for methods is consistent:
/// - find_<construct>: Returns a reference to the construct with the given
///   name.
impl Component {
    /// Construct a new Component with the given `name` and signature fields.
    pub fn new<S, N>(name: S, ports: Vec<(N, u64, Direction)>) -> Self
    where
        S: AsRef<str>,
        N: AsRef<str>,
    {
        let this_sig = Builder::cell_from_signature(
            THIS_ID.into(),
            CellType::ThisComponent,
            ports
                .into_iter()
                // Reverse the port directions inside the component.
                .map(|(name, w, d)| (name.as_ref().into(), w, d.reverse()))
                .collect(),
        );

        Component {
            name: name.as_ref().into(),
            signature: this_sig,
            cells: vec![],
            groups: vec![],
            continuous_assignments: vec![],
            control: Rc::new(RefCell::new(Control::empty())),
            namegen: utils::NameGenerator::default(),
        }
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_group<S>(&self, name: &S) -> Option<RRC<Group>>
    where
        S: Clone + AsRef<str>,
    {
        self.groups
            .iter()
            .find(|&g| g.borrow().name == *name)
            .map(|r| Rc::clone(r))
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell<S>(&self, name: &S) -> Option<RRC<Cell>>
    where
        S: Clone + AsRef<str>,
    {
        self.cells
            .iter()
            .find(|&g| g.borrow().name == *name)
            .map(|r| Rc::clone(r))
    }

    /// Construct a non-conflicting name using the Component's namegenerator.
    pub fn generate_name<S>(&mut self, prefix: S) -> Id
    where
        S: Into<Id> + ToString,
    {
        self.namegen.gen_name(prefix)
    }
}
