use super::{
    Assignment, Cell, CellType, Control, Direction, Group, Guard, Port,
    PortParent, RRC,
};
use crate::frontend::ast::Id;
use std::cell::RefCell;
use std::collections::HashMap;
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
/// - build_<construct>: Create and return a reference to the
///   construct.
impl Component {
    /// Return a reference to the group with `name` if present.
    pub fn find_group(&self, name: Id) -> Option<RRC<Group>> {
        self.groups
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell(&self, name: Id) -> Option<RRC<Cell>> {
        self.cells
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Construct a new group using `name` and `attributes`.
    /// Returns a reference to the group.
    pub fn build_group(
        &self,
        name: String,
        attributes: HashMap<String, u64>,
    ) -> RRC<Group> {
        let group = Rc::new(RefCell::new(Group {
            name: name.into(),
            attributes,
            holes: vec![],
            assignments: vec![],
        }));

        // Add default holes to the group.
        for (name, width) in vec![("go", 1), ("done", 1)] {
            let hole = Rc::new(RefCell::new(Port {
                name: name.into(),
                width,
                direction: Direction::Inout,
                parent: PortParent::Group(Rc::downgrade(&group)),
            }));
            group.borrow_mut().holes.push(hole);
        }

        group
    }

    /// Construct an assignment.
    pub fn build_assignment(
        &self,
        dst: RRC<Port>,
        src: RRC<Port>,
        guard: Option<Guard>,
    ) -> Assignment {
        Assignment { dst, src, guard }
    }

    /// Return reference for a constant cell associated with the (val, width)
    /// pair.
    /// If the constant does not exist, it is added to the Context.
    pub fn build_constant(&mut self, val: u64, width: u64) -> RRC<Cell> {
        let name = Cell::constant_name(val, width);
        // If this constant has already been instantiated, return the relevant
        // cell.
        if let Some(cell) = self.cells.iter().find(|&c| c.borrow().name == name)
        {
            return Rc::clone(cell);
        }

        // Construct this cell if it's not already present in the context.
        let cell = Component::cell_from_signature(
            name.clone(),
            CellType::Constant { val, width },
            vec![],
            vec![("out".into(), width)],
        );

        // Add constant to the Component.
        self.cells.push(Rc::clone(&cell));

        cell
    }

    /// Construct a cell from input/output signature.
    /// Input and output port definition in the form (name, width).
    pub(super) fn cell_from_signature(
        name: Id,
        typ: CellType,
        inputs: Vec<(Id, u64)>,
        outputs: Vec<(Id, u64)>,
    ) -> RRC<Cell> {
        let cell = Rc::new(RefCell::new(Cell {
            name,
            ports: vec![],
            prototype: typ,
        }));
        // Construct ports
        for (name, width) in inputs {
            let port = Rc::new(RefCell::new(Port {
                name,
                width,
                direction: Direction::Input,
                parent: PortParent::Cell(Rc::downgrade(&cell)),
            }));
            cell.borrow_mut().ports.push(port);
        }
        for (name, width) in outputs {
            let port = Rc::new(RefCell::new(Port {
                name,
                width,
                direction: Direction::Output,
                parent: PortParent::Cell(Rc::downgrade(&cell)),
            }));
            cell.borrow_mut().ports.push(port);
        }
        cell
    }
}
