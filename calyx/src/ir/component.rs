use super::{Assignment, Cell, Group, Id, RRC, Control, Builder, CellType};
use std::rc::Rc;
use std::cell::RefCell;

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
    /// Construct a new Component with the given `name` and signature fields.
    pub fn new<S, I, O>(
        name: S,
        inputs: Vec<(I, u64)>,
        outputs: Vec<(O, u64)>,
    ) -> Self
    where
        S: AsRef<str>,
        I: AsRef<str>,
        O: AsRef<str>,
    {
        let this_sig = Builder::cell_from_signature(
            "this".into(),
            CellType::ThisComponent,
            inputs
                .into_iter()
                .map(|(name, width)| (name.as_ref().into(), width))
                .collect(),
            outputs
                .into_iter()
                .map(|(name, width)| (name.as_ref().into(), width))
                .collect(),
        );
        Component {
            name: name.as_ref().into(),
            signature: this_sig,
            cells: vec![],
            groups: vec![],
            continuous_assignments: vec![],
            control: Rc::new(RefCell::new(Control::empty())),
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
}
