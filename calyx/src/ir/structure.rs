//! Representation for structure (wires and cells) in a Calyx program.

use super::{Attributes, GetAttributes, Guard, Id, PortDef, RRC, WRC};
use itertools::Itertools;
use serde::Serialize;
use serde_with::{serde_as, SerializeAs};
use smallvec::{smallvec, SmallVec};
use std::hash::Hash;
use std::rc::Rc;

/// Direction of a port on a cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// Attributes associated with this port.
    pub attributes: Attributes,
}

pub struct SerPortRef;
impl SerializeAs<RRC<Port>> for SerPortRef {
    fn serialize_as<S>(
        value: &RRC<Port>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().name.serialize(serializer)
    }
}

/// Canonical name of a Port
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Canonical(pub Id, pub Id);

impl std::fmt::Display for Canonical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

impl Port {
    /// Checks if this port is a hole
    pub fn is_hole(&self) -> bool {
        matches!(&self.parent, PortParent::Group(_))
    }

    /// Returns the parent of the [Port] which must be [Cell]. Throws an error
    /// otherwise.
    pub fn cell_parent(&self) -> RRC<Cell> {
        if let PortParent::Cell(cell_wref) = &self.parent {
            return cell_wref.upgrade();
        }
        unreachable!("This port should have a cell parent")
    }

    /// Checks if this port is a constant of value: `val`.
    pub fn is_constant(&self, val: u64, width: u64) -> bool {
        if let PortParent::Cell(cell) = &self.parent {
            match cell.upgrade().borrow().prototype {
                CellType::Constant { val: v, width: w } => {
                    v == val && width == w
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Gets name of parent object.
    pub fn get_parent_name(&self) -> Id {
        match &self.parent {
            PortParent::Cell(cell) => cell.upgrade().borrow().name,
            PortParent::Group(group) => group.upgrade().borrow().name,
        }
    }

    /// Get the canonical representation for this Port.
    pub fn canonical(&self) -> Canonical {
        Canonical(self.get_parent_name(), self.name)
    }
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.get_parent_name() == other.get_parent_name()
            && self.name == other.name
    }
}

impl Eq for Port {}

/// Wraps generic iterators over ports to allow functions to build and return port iterators in
/// different ways.
pub struct PortIterator<'a> {
    pub port_iter: Box<dyn Iterator<Item = RRC<Port>> + 'a>,
}

impl PortIterator<'_> {
    /// Returns an empty iterator over ports.
    pub fn empty() -> Self {
        PortIterator {
            port_iter: Box::new(std::iter::empty()),
        }
    }
}

impl Iterator for PortIterator<'_> {
    type Item = RRC<Port>;

    fn next(&mut self) -> Option<Self::Item> {
        self.port_iter.next()
    }
}

/// Alias for bindings
pub type Binding = SmallVec<[(Id, u64); 5]>;

pub struct SerBinding;
impl SerializeAs<Binding> for SerBinding {
    fn serialize_as<S>(
        value: &Binding,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(value.iter())
    }
}

/// The type for a Cell
#[serde_as]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize)]
pub enum CellType {
    /// Cell constructed using a primitive definition
    Primitive {
        /// Name of the primitive cell used to instantiate this cell.
        name: Id,
        /// Bindings for the parameters. Uses Vec to retain the input order.
        #[serde_as(as = "SerBinding")]
        param_binding: Box<Binding>,
        /// True iff this is a combinational primitive
        is_comb: bool,
    },
    /// Cell constructed using a Calyx component
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

impl CellType {
    /// Return the name associated with this CellType is present
    pub fn get_name(&self) -> Option<Id> {
        match self {
            CellType::Primitive { name, .. } | CellType::Component { name } => {
                Some(*name)
            }
            CellType::ThisComponent | CellType::Constant { .. } => None,
        }
    }

    /// Generate string representation of CellType appropriate for error messages.
    pub fn surface_name(&self) -> Option<String> {
        match self {
            CellType::Primitive {
                name,
                param_binding,
                ..
            } => Some(format!(
                "{}({})",
                name,
                param_binding.iter().map(|(_, v)| v.to_string()).join(", ")
            )),
            CellType::Component { name } => Some(name.to_string()),
            CellType::ThisComponent | CellType::Constant { .. } => None,
        }
    }
}

/// Represents an instantiated cell.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct Cell {
    /// Name of this cell.
    name: Id,
    /// Ports on this cell
    #[serde_as(as = "SerVecPortRef")]
    pub ports: SmallVec<[RRC<Port>; 10]>,
    /// Underlying type for this cell
    pub prototype: CellType,
    /// Attributes for this group.
    pub attributes: Attributes,
    /// Whether the cell is external
    reference: bool,
}

impl GetAttributes for Cell {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Cell {
    /// Construct a cell
    pub fn new(name: Id, prototype: CellType) -> Self {
        Self {
            name,
            ports: smallvec![],
            prototype,
            attributes: Attributes::default(),
            reference: false,
        }
    }

    ///Get a boolean describing whether the cell is external.
    pub fn is_reference(&self) -> bool {
        self.reference
    }

    ///Set the external field
    pub(super) fn set_reference(&mut self, reference: bool) -> bool {
        self.reference = reference;
        self.reference
    }

    /// Get a reference to the named port if it exists.
    pub fn find<S>(&self, name: S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display + Clone,
        Id: PartialEq<S>,
    {
        self.ports
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(Rc::clone)
    }

    /// Get a reference to the first port that has the attribute `attr`.
    pub fn find_with_attr<S>(&self, attr: S) -> Option<RRC<Port>>
    where
        S: Into<Id>,
    {
        let key = attr.into();
        self.ports
            .iter()
            .find(|&g| g.borrow().attributes.has(key))
            .map(Rc::clone)
    }

    /// Return all ports that have the attribute `attr`.
    pub fn find_all_with_attr<S>(
        &self,
        attr: S,
    ) -> impl Iterator<Item = RRC<Port>> + '_
    where
        S: Into<Id>,
    {
        let key = attr.into();
        self.ports
            .iter()
            .filter(move |&p| p.borrow().attributes.has(key))
            .map(Rc::clone)
    }

    /// Get a reference to the named port and throw an error if it doesn't
    /// exist.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone,
        Id: PartialEq<S>,
    {
        self.find(name.clone()).unwrap_or_else(|| {
            panic!(
                "Port `{name}' not found on cell `{}'. Known ports are: {}",
                self.name,
                self.ports
                    .iter()
                    .map(|p| p.borrow().name.to_string())
                    .join(",")
            )
        })
    }

    /// Returns true iff this cell is an instance of a Calyx-defined component.
    pub fn is_component(&self) -> bool {
        matches!(&self.prototype, CellType::Component { .. })
    }

    /// Returns true iff this cell is the signature of the current component
    pub fn is_this(&self) -> bool {
        matches!(&self.prototype, CellType::ThisComponent)
    }

    /// Returns true if this is an instance of a primitive. If the optional name is provided then
    /// only returns true if the primitive has the given name.
    pub fn is_primitive<S>(&self, prim: Option<S>) -> bool
    where
        Id: PartialEq<S>,
    {
        match &self.prototype {
            CellType::Primitive { name, .. } => {
                prim.as_ref().map(|p| name == p).unwrap_or(true)
            }
            _ => false,
        }
    }

    /// Get a reference to the first port with the attribute `attr` and throw an error if none
    /// exist.
    pub fn get_with_attr<S>(&self, attr: S) -> RRC<Port>
    where
        S: Into<Id>,
    {
        let key = attr.into();
        self.find_with_attr(key).unwrap_or_else(|| {
            panic!(
                "Port with attribute `{}' not found on cell `{}'",
                key, self.name,
            )
        })
    }

    /// Returns the name of the component that is this cells type.
    pub fn type_name(&self) -> Option<Id> {
        self.prototype.get_name()
    }

    /// Get parameter binding from the prototype used to build this cell.
    pub fn get_parameter<S>(&self, param: S) -> Option<u64>
    where
        Id: PartialEq<S>,
    {
        match &self.prototype {
            CellType::Primitive { param_binding, .. } => param_binding
                .iter()
                .find(|(key, _)| *key == param)
                .map(|(_, val)| *val),
            CellType::Component { .. } => None,
            CellType::ThisComponent => None,
            CellType::Constant { .. } => None,
        }
    }

    /// Return the canonical name for the cell generated to represent this
    /// (val, width) constant.
    pub fn constant_name(val: u64, width: u64) -> Id {
        format!("_{}_{}", val, width).into()
    }

    /// Return the value associated with this attribute key.
    pub fn get_attribute<S>(&self, attr: S) -> Option<&u64>
    where
        S: Into<Id>,
    {
        let key = attr.into();
        self.attributes.get(key)
    }

    /// Add a new attribute to the group.
    pub fn add_attribute<S>(&mut self, attr: S, value: u64)
    where
        S: Into<String>,
    {
        self.attributes.insert(attr.into(), value);
    }

    /// Grants immutable access to the name of this cell.
    pub fn name(&self) -> Id {
        self.name
    }

    /// Returns a reference to all [super::Port] attached to this cells.
    pub fn ports(&self) -> &SmallVec<[RRC<Port>; 10]> {
        &self.ports
    }

    // Get the signature of this cell as a vector. Each element corresponds to a port in the Cell.
    pub fn get_signature(&self) -> Vec<PortDef<u64>> {
        self.ports
            .iter()
            .map(|port_ref| {
                let port = port_ref.borrow();
                PortDef {
                    name: port.name,
                    width: port.width,
                    direction: port.direction.clone(),
                    attributes: port.attributes.clone(),
                }
            })
            .collect()
    }
}

pub struct SerCellRef;
impl SerializeAs<RRC<Cell>> for SerCellRef {
    fn serialize_as<S>(
        value: &RRC<Cell>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.borrow().name().serialize(serializer)
    }
}

/// Represents a guarded assignment in the program
#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct Assignment {
    /// The destination for the assignment.
    #[serde_as(as = "SerPortRef")]
    pub dst: RRC<Port>,

    /// The source for the assignment.
    #[serde_as(as = "SerPortRef")]
    pub src: RRC<Port>,

    /// The guard for this assignment.
    pub guard: Box<Guard>,

    /// Attributes for this assignment.
    pub attributes: Attributes,
}

impl Assignment {
    /// Apply function `f` to each port contained within the assignment and
    /// replace the port with the generated value if not None.
    pub fn for_each_port<F>(&mut self, mut f: F)
    where
        F: FnMut(&RRC<Port>) -> Option<RRC<Port>>,
    {
        if let Some(new_src) = f(&self.src) {
            self.src = new_src;
        }
        if let Some(new_dst) = f(&self.dst) {
            self.dst = new_dst;
        }
        self.guard.for_each(&mut |port| f(&port).map(Guard::port))
    }
}

pub struct SerVecPortRef;
impl<const N: usize> SerializeAs<SmallVec<[RRC<Port>; N]>> for SerVecPortRef {
    fn serialize_as<S>(
        value: &SmallVec<[RRC<Port>; N]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(value.iter().map(|v| v.borrow().name))
    }
}

/// A Group of assignments that perform a logical action.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct Group {
    /// Name of this group
    name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment>,

    /// Holes for this group
    #[serde_as(as = "SerVecPortRef")]
    pub holes: SmallVec<[RRC<Port>; 3]>,

    /// Attributes for this group.
    pub attributes: Attributes,
}
impl Group {
    pub fn new(name: Id) -> Self {
        Self {
            name,
            assignments: vec![],
            holes: smallvec![],
            attributes: Attributes::default(),
        }
    }

    /// Get a reference to the named hole if it exists.
    pub fn find<S>(&self, name: S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display,
        Id: PartialEq<S>,
    {
        self.holes
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(Rc::clone)
    }

    /// Get a reference to the named hole or panic.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone,
        Id: PartialEq<S>,
    {
        self.find(name.clone()).unwrap_or_else(|| {
            panic!("Hole `{name}' not found on group `{}'", self.name)
        })
    }

    /// Returns the index to the done assignment in the group.
    fn find_done_cond(&self) -> usize {
        self.assignments
            .iter()
            .position(|assign| {
                let dst = assign.dst.borrow();
                dst.is_hole() && dst.name == "done"
            })
            .unwrap_or_else(|| {
                panic!("Group `{}' has no done condition", self.name)
            })
    }

    /// Returns a reference to the assignment in the group that writes to the done condition.
    pub fn done_cond(&self) -> &Assignment {
        let idx = self.find_done_cond();
        &self.assignments[idx]
    }

    /// Returns a mutable reference to the assignment in the group that writes to the done
    /// condition.
    pub fn done_cond_mut(&mut self) -> &mut Assignment {
        let idx = self.find_done_cond();
        &mut self.assignments[idx]
    }

    /// The name of this group.
    #[inline]
    pub fn name(&self) -> Id {
        self.name
    }

    /// The attributes of this group.
    #[inline]
    pub fn get_attributes(&self) -> Option<&Attributes> {
        Some(&self.attributes)
    }
}

/// A combinational group.
/// A combinational group does not have any holes and should only contain assignments that should
/// will be combinationally active
#[derive(Debug, Serialize)]
pub struct CombGroup {
    /// Name of this group
    pub(super) name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment>,

    /// Attributes for this group.
    pub attributes: Attributes,
}
impl CombGroup {
    /// The name of this group.
    #[inline]
    pub fn name(&self) -> Id {
        self.name
    }

    /// The attributes of this group.
    #[inline]
    pub fn get_attributes(&self) -> Option<&Attributes> {
        Some(&self.attributes)
    }
}

/// A trait representing something in the IR that has a name.
pub trait GetName {
    /// Return a reference to the object's name
    fn name(&self) -> Id;
}

impl GetName for Cell {
    fn name(&self) -> Id {
        self.name()
    }
}

impl GetName for Group {
    fn name(&self) -> Id {
        self.name()
    }
}

impl GetName for CombGroup {
    fn name(&self) -> Id {
        self.name()
    }
}

/// A utility trait representing the ability to clone the name of an object.
/// Automatically definied for anything that implements GetName
pub trait CloneName {
    /// Returns a clone of the object's name
    fn clone_name(&self) -> Id;
}

impl<T: GetName> CloneName for T {
    fn clone_name(&self) -> Id {
        self.name()
    }
}
