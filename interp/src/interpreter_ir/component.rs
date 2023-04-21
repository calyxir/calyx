use super::control::Control;
use calyx_ir::Component as CalyxComponent;
use calyx_ir::{
    Assignment, Attributes, Cell, CombGroup, Group, Id, IdList, Nothing, RRC,
};
use std::rc::Rc;

#[derive(Debug)]
pub struct Component {
    /// Name of the component.
    pub name: Id,
    /// The input/output signature of this component.
    pub signature: RRC<Cell>,
    /// The cells instantiated for this component.
    pub cells: IdList<Cell>,
    /// Groups of assignment wires.
    pub groups: IdList<Group>,
    /// Groups of assignment wires.
    pub comb_groups: IdList<CombGroup>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Rc<Vec<Assignment<Nothing>>>,
    /// The control program for this component.
    pub control: Control,
    /// Attributes for this component
    pub attributes: Attributes,
}

impl From<CalyxComponent> for Component {
    fn from(cc: CalyxComponent) -> Self {
        Self {
            name: cc.name,
            signature: cc.signature,
            cells: cc.cells,
            groups: cc.groups,
            comb_groups: cc.comb_groups,
            continuous_assignments: Rc::new(cc.continuous_assignments),
            control: Rc::try_unwrap(cc.control).unwrap().into_inner().into(),
            attributes: cc.attributes,
        }
    }
}

impl Component {
    /// Return a reference to the cell with `name` if present.
    pub fn find_cell<S>(&self, name: S) -> Option<RRC<Cell>>
    where
        S: Clone + Into<Id>,
    {
        self.cells.find(name)
    }
}
