//! Representation for structure (wires and cells) in a Calyx program.

use crate::guard::StaticTiming;
use crate::Nothing;

use super::{
    Attributes, Direction, GetAttributes, Guard, Id, PortDef, RRC, WRC,
};
use calyx_frontend::Attribute;
use calyx_utils::{CalyxResult, Error, GetName};
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};
use std::hash::Hash;
use std::rc::Rc;

/// Ports can come from Cells or Groups
#[derive(Debug, Clone)]
pub enum PortParent {
    Cell(WRC<Cell>),
    Group(WRC<Group>),
    StaticGroup(WRC<StaticGroup>),
}

/// Represents a port on a cell.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
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

/// Canonical name of a Port
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Canonical {
    pub cell: Id,
    pub port: Id,
}

impl Canonical {
    pub const fn new(cell: Id, port: Id) -> Self {
        Self { cell, port }
    }
}

impl std::fmt::Display for Canonical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.cell, self.port)
    }
}

impl Port {
    /// Checks if this port is a hole
    pub fn is_hole(&self) -> bool {
        matches!(&self.parent, PortParent::Group(_))
            || matches!(&self.parent, PortParent::StaticGroup(_))
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
            PortParent::StaticGroup(group) => group.upgrade().borrow().name,
        }
    }

    /// Checks if parent is combinational component
    pub fn parent_is_comb(&self) -> bool {
        match &self.parent {
            PortParent::Cell(cell) => cell.upgrade().borrow().is_comb_cell(),
            _ => false,
        }
    }

    /// Get the canonical representation for this Port.
    pub fn canonical(&self) -> Canonical {
        Canonical {
            cell: self.get_parent_name(),
            port: self.name,
        }
    }

    /// Returns the value of an attribute if present
    pub fn get_attribute<A>(&self, attr: A) -> Option<u64>
    where
        A: Into<Attribute>,
    {
        self.get_attributes().get(attr)
    }

    /// Returns true if the node has a specific attribute
    pub fn has_attribute<A>(&self, attr: A) -> bool
    where
        A: Into<Attribute>,
    {
        self.get_attributes().has(attr)
    }
}

impl GetAttributes for Port {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn get_mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
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
    port_iter: Box<dyn Iterator<Item = RRC<Port>> + 'a>,
}

impl<'a> PortIterator<'a> {
    /// Construct a new PortIterator from an iterator over ports.
    pub fn new<T>(iter: T) -> Self
    where
        T: Iterator<Item = RRC<Port>> + 'a,
    {
        PortIterator {
            port_iter: Box::new(iter),
        }
    }

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

/// The type for a Cell
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub enum CellType {
    /// Cell constructed using a primitive definition
    Primitive {
        /// Name of the primitive cell used to instantiate this cell.
        name: Id,
        /// Bindings for the parameters. Uses Vec to retain the input order.
        param_binding: Box<Binding>,
        /// True iff this is a combinational primitive
        is_comb: bool,
        /// (Optional) latency of the primitive
        latency: Option<std::num::NonZeroU64>,
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
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Cell {
    /// Name of this cell.
    name: Id,
    /// Ports on this cell
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
    pub fn set_reference(&mut self, reference: bool) -> bool {
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

    /// Return all ports that have the attribute `attr`.
    pub fn find_all_with_attr<A>(
        &self,
        attr: A,
    ) -> impl Iterator<Item = RRC<Port>> + '_
    where
        A: Into<Attribute>,
    {
        let attr = attr.into();
        self.ports
            .iter()
            .filter(move |&p| p.borrow().attributes.has(attr))
            .map(Rc::clone)
    }

    /// Return the unique port with the given attribute.
    /// If multiple ports have the same attribute, then we panic.
    /// If there are not ports with the give attribute, then we return None.
    pub fn find_unique_with_attr<A>(
        &self,
        attr: A,
    ) -> CalyxResult<Option<RRC<Port>>>
    where
        A: Into<Attribute>,
    {
        let attr = attr.into();
        let mut ports = self.find_all_with_attr(attr);
        if let Some(port) = ports.next() {
            if ports.next().is_some() {
                Err(Error::malformed_structure(format!(
                    "Multiple ports with attribute `{}` found on cell `{}`",
                    attr, self.name
                )))
            } else {
                Ok(Some(port))
            }
        } else {
            Ok(None)
        }
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

    /// Get the unique port with the given attribute.
    /// Panic if no port with the attribute is found and returns an error if multiple ports with the attribute are found.
    pub fn get_unique_with_attr<A>(&self, attr: A) -> CalyxResult<RRC<Port>>
    where
        A: Into<Attribute> + std::fmt::Display + Copy,
    {
        Ok(self.find_unique_with_attr(attr)?.unwrap_or_else(|| {
            panic!(
                "Port with attribute `{attr}' not found on cell `{}'",
                self.name,
            )
        }))
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
    pub fn get_attribute<A: Into<Attribute>>(&self, attr: A) -> Option<u64> {
        self.attributes.get(attr.into())
    }

    /// Add a new attribute to the group.
    pub fn add_attribute<A: Into<Attribute>>(&mut self, attr: A, value: u64) {
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
                PortDef::new(
                    port.name,
                    port.width,
                    port.direction.clone(),
                    port.attributes.clone(),
                )
            })
            .collect()
    }

    // returns true if cell is comb, false otherwise
    // note that this component/component cannot be combinational
    // XXX(rachit): Combinational components are now supported so this function returns
    // the wrong answer when the parent is a combinational component
    pub fn is_comb_cell(&self) -> bool {
        match self.prototype {
            CellType::Primitive { is_comb, .. } => is_comb,
            _ => false,
        }
    }
}

/// Represents a guarded assignment in the program
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Assignment<T> {
    /// The destination for the assignment.
    pub dst: RRC<Port>,

    /// The source for the assignment.
    pub src: RRC<Port>,

    /// The guard for this assignment.
    pub guard: Box<Guard<T>>,

    /// Attributes for this assignment.
    pub attributes: Attributes,
}

impl<T> Assignment<T> {
    /// Build a new unguarded assignment
    pub fn new(dst: RRC<Port>, src: RRC<Port>) -> Self {
        assert!(
            dst.borrow().direction == Direction::Input,
            "{} is not in input port",
            dst.borrow().canonical()
        );
        assert!(
            src.borrow().direction == Direction::Output,
            "{} is not in output port",
            src.borrow().canonical()
        );
        Self {
            dst,
            src,
            guard: Box::new(Guard::True),
            attributes: Attributes::default(),
        }
    }

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

impl From<Assignment<Nothing>> for Assignment<StaticTiming> {
    /// Turns a normal assignment into a static assignment
    fn from(assgn: Assignment<Nothing>) -> Assignment<StaticTiming> {
        Assignment {
            dst: Rc::clone(&assgn.dst),
            src: Rc::clone(&assgn.src),
            guard: Box::new(Guard::from(*assgn.guard)),
            attributes: assgn.attributes,
        }
    }
}

impl<StaticTiming> Assignment<StaticTiming> {
    /// Apply function `f` to each port contained within the assignment and
    /// replace the port with the generated value if not None.
    pub fn for_each_interval<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut StaticTiming) -> Option<Guard<StaticTiming>>,
    {
        self.guard.for_each_info(&mut |interval| f(interval))
    }
}

/// A Group of assignments that perform a logical action.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Group {
    /// Name of this group
    name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment<Nothing>>,

    /// Holes for this group
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
    pub fn done_cond(&self) -> &Assignment<Nothing> {
        let idx = self.find_done_cond();
        &self.assignments[idx]
    }

    /// Returns a mutable reference to the assignment in the group that writes to the done
    /// condition.
    pub fn done_cond_mut(&mut self) -> &mut Assignment<Nothing> {
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

    pub fn remove_attribute(&mut self, attr: Attribute) {
        self.attributes.remove(attr);
    }
}

/// A Group of assignments that perform a logical action.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct StaticGroup {
    /// Name of this group
    name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment<StaticTiming>>,

    /// Holes for this group
    pub holes: SmallVec<[RRC<Port>; 3]>,

    /// Attributes for this group.
    pub attributes: Attributes,

    /// Latency of static group
    pub latency: u64,
}

///implement the StaticGroup struct
impl StaticGroup {
    pub fn new(name: Id, latency: u64) -> Self {
        Self {
            name,
            assignments: vec![],
            holes: smallvec![],
            attributes: Attributes::default(),
            latency,
        }
    }

    pub fn get_latency(&self) -> u64 {
        self.latency
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

    pub fn remove_attribute(&mut self, attr: Attribute) {
        self.attributes.remove(attr);
    }
}

/// A combinational group.
/// A combinational group does not have any holes and should only contain assignments that should
/// will be combinationally active
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct CombGroup {
    /// Name of this group
    pub(super) name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment<Nothing>>,

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

impl GetName for StaticGroup {
    fn name(&self) -> Id {
        self.name()
    }
}
