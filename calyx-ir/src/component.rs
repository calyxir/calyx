use super::{
    Assignment, Attribute, Attributes, BoolAttr, Builder, Cell, CellType,
    CombGroup, Control, Direction, GetName, Group, Id, NumAttr, PortDef,
    StaticGroup, RRC,
};
use crate::guard::StaticTiming;
use crate::Nothing;
use calyx_utils::NameGenerator;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;
use std::iter::Extend;
use std::num::NonZeroU64;
use std::rc::Rc;

/// The default name of the signature cell in a component.
/// In general, this should not be used by anything.
const THIS_ID: &str = "_this";

/// Interface ports that must be present on every component
const INTERFACE_PORTS: [(Attribute, u64, Direction); 4] = [
    (Attribute::Num(NumAttr::Go), 1, Direction::Input),
    (Attribute::Bool(BoolAttr::Clk), 1, Direction::Input),
    (Attribute::Bool(BoolAttr::Reset), 1, Direction::Input),
    (Attribute::Num(NumAttr::Done), 1, Direction::Output),
];

/// In memory representation of a Component.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Component {
    /// Name of the component.
    pub name: Id,
    /// The input/output signature of this component.
    pub signature: RRC<Cell>,
    /// The cells instantiated for this component.
    pub cells: IdList<Cell>,
    /// Groups of assignment wires.
    pub groups: IdList<Group>,
    /// Groups of assignment wires
    pub static_groups: IdList<StaticGroup>,
    /// Groups of assignment wires.
    pub comb_groups: IdList<CombGroup>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Vec<Assignment<Nothing>>,
    /// The control program for this component.
    pub control: RRC<Control>,
    /// Attributes for this component
    pub attributes: Attributes,
    /// True iff component is combinational
    pub is_comb: bool,
    /// (Optional) latency of component, if it is static
    pub latency: Option<NonZeroU64>,

    ///// Internal structures
    /// Namegenerator that contains the names currently defined in this
    /// component (cell and group names).
    #[cfg_attr(feature = "serialize", serde(skip))]
    namegen: NameGenerator,
}

/// Builder methods for extracting and construction IR nodes.
/// The naming scheme for methods is consistent:
/// - find_<construct>: Returns a reference to the construct with the given
///   name.
impl Component {
    /// Extend the signature with interface ports if they are missing.
    pub(super) fn extend_signature(sig: &mut Vec<PortDef<u64>>) {
        let port_names: HashSet<_> = sig.iter().map(|pd| pd.name()).collect();
        let mut namegen = NameGenerator::with_prev_defined_names(port_names);
        for (attr, width, direction) in INTERFACE_PORTS.iter() {
            // Check if there is already another interface port defined for the
            // component
            if !sig.iter().any(|pd| pd.attributes.has(*attr)) {
                let mut attributes = Attributes::default();
                attributes.insert(*attr, 1);
                let name = Id::from(attr.to_string());
                sig.push(PortDef::new(
                    namegen.gen_name(name.to_string()),
                    *width,
                    direction.clone(),
                    attributes,
                ));
            }
        }
    }

    /// Construct a new Component with the given `name` and ports.
    ///
    /// * If `has_interface` is true, then we do not add `@go` and `@done` ports.
    ///   This will usually happen with the component is marked with [super::BoolAttr::Nointerface].
    /// * If `is_comb` is set, then this is a combinational component and cannot use `group` or `control` constructs.
    /// * If `latency` is set, then this is a static component with the given latency. A combinational component cannot have a latency.
    pub fn new<S>(
        name: S,
        mut ports: Vec<PortDef<u64>>,
        has_interface: bool,
        is_comb: bool,
        latency: Option<NonZeroU64>,
    ) -> Self
    where
        S: Into<Id>,
    {
        if has_interface {
            // Add interface ports if missing
            Self::extend_signature(&mut ports);
        }

        let prev_names: HashSet<_> = ports.iter().map(|pd| pd.name()).collect();

        let this_sig = Builder::cell_from_signature(
            THIS_ID.into(),
            CellType::ThisComponent,
            ports
                .into_iter()
                // Reverse the port directions inside the component.
                .map(|pd| {
                    PortDef::new(
                        pd.name(),
                        pd.width,
                        pd.direction.reverse(),
                        pd.attributes,
                    )
                })
                .collect(),
        );

        Component {
            name: name.into(),
            signature: this_sig,
            cells: IdList::default(),
            groups: IdList::default(),
            static_groups: IdList::default(),
            comb_groups: IdList::default(),
            continuous_assignments: vec![],
            control: super::rrc(Control::empty()),
            namegen: NameGenerator::with_prev_defined_names(prev_names),
            attributes: Attributes::default(),
            is_comb,
            // converting from NonZeroU64 to u64. May want to keep permanently as NonZeroU64
            // in the future, but rn it's probably easier to keep as u64
            latency,
        }
    }

    pub(super) fn add_names(&mut self, names: HashSet<Id>) {
        self.namegen.add_names(names)
    }

    /// gets the component's groups
    pub fn get_groups(&self) -> &IdList<Group> {
        &self.groups
    }

    /// gets the component's static groups
    pub fn get_static_groups(&self) -> &IdList<StaticGroup> {
        &self.static_groups
    }

    /// gets the component's groups
    pub fn get_groups_mut(&mut self) -> &mut IdList<Group> {
        &mut self.groups
    }

    /// gets the component's groups
    pub fn get_static_groups_mut(&mut self) -> &mut IdList<StaticGroup> {
        &mut self.static_groups
    }

    /// gets the component's groups
    pub fn set_groups(&mut self, groups: IdList<Group>) {
        self.groups = groups
    }

    /// gets the component's groups
    pub fn set_static_groups(&mut self, static_groups: IdList<StaticGroup>) {
        self.static_groups = static_groups
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_group<S>(&self, name: S) -> Option<RRC<Group>>
    where
        S: Into<Id>,
    {
        self.groups.find(name)
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_static_group<S>(&self, name: S) -> Option<RRC<StaticGroup>>
    where
        S: Into<Id>,
    {
        self.static_groups.find(name)
    }

    /// Return a refernece to a combination group with `name` if present.
    pub fn find_comb_group<S>(&self, name: S) -> Option<RRC<CombGroup>>
    where
        S: Into<Id>,
    {
        self.comb_groups.find(name)
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell<S>(&self, name: S) -> Option<RRC<Cell>>
    where
        S: Into<Id>,
    {
        self.cells.find(name)
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_guaranteed_cell<S>(&self, name: S) -> RRC<Cell>
    where
        S: Into<Id> + std::fmt::Debug + Copy,
    {
        self.cells.find(name).unwrap_or_else(|| {
            unreachable!(
                "called find_certain_cell on {:?} but it wasn't found",
                name
            )
        })
    }

    /// Construct a non-conflicting name using the Component's namegenerator.
    pub fn generate_name<S>(&mut self, prefix: S) -> Id
    where
        S: Into<Id>,
    {
        self.namegen.gen_name(prefix)
    }

    /// Check whether this component is purely structural, i.e. has no groups or control
    pub fn is_structural(&self) -> bool {
        self.groups.is_empty()
            && self.comb_groups.is_empty()
            && self.static_groups.is_empty()
            && self.control.borrow().is_empty()
    }

    /// Check whether this is a static component.
    /// A static component is a component which has a latency field.
    pub fn is_static(&self) -> bool {
        self.latency.is_some()
    }

    /// Apply function to all assignments within static groups.
    pub fn for_each_static_assignment<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Assignment<StaticTiming>),
    {
        for group_ref in self.get_static_groups().iter() {
            let mut assigns =
                group_ref.borrow_mut().assignments.drain(..).collect_vec();
            for assign in &mut assigns {
                f(assign)
            }
            group_ref.borrow_mut().assignments = assigns;
        }
    }

    /// Apply function on all non-static assignments contained within the component.
    pub fn for_each_assignment<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Assignment<Nothing>),
    {
        // Detach assignments from the group so that ports that use group
        // `go` and `done` condition can access the parent group.
        for group_ref in self.groups.iter() {
            let mut assigns =
                group_ref.borrow_mut().assignments.drain(..).collect_vec();
            for assign in &mut assigns {
                f(assign)
            }
            group_ref.borrow_mut().assignments = assigns;
        }
        for group_ref in self.comb_groups.iter() {
            let mut assigns =
                group_ref.borrow_mut().assignments.drain(..).collect_vec();
            for assign in &mut assigns {
                f(assign)
            }
            group_ref.borrow_mut().assignments = assigns;
        }
        self.continuous_assignments.iter_mut().for_each(f);
    }

    /// Iterate over all non-static assignments contained within the component.
    pub fn iter_assignments<F>(&self, mut f: F)
    where
        F: FnMut(&Assignment<Nothing>),
    {
        for group_ref in self.groups.iter() {
            for assign in &group_ref.borrow().assignments {
                f(assign)
            }
        }
        for group_ref in self.comb_groups.iter() {
            for assign in &group_ref.borrow().assignments {
                f(assign)
            }
        }
        self.continuous_assignments.iter().for_each(f);
    }

    /// Iterate over all static assignments contained within the component
    pub fn iter_static_assignments<F>(&self, mut f: F)
    where
        F: FnMut(&Assignment<StaticTiming>),
    {
        for group_ref in self.get_static_groups().iter() {
            for assign in &group_ref.borrow().assignments {
                f(assign)
            }
        }
    }
}

/// A wrapper struct exposing an ordered collection of named entities within an
/// RRC with deterministic iteration and constant-time look-up on names
/// directly. The struct assumes that the name of an entity cannot change. Doing
/// so will introduce incorrect results for look-ups.
#[derive(Debug)]
pub struct IdList<T: GetName>(LinkedHashMap<Id, RRC<T>>);

/// Simple iter impl delegating to the [`Values`](linked_hash_map::Values).
impl<'a, T: GetName> IntoIterator for &'a IdList<T> {
    type Item = &'a RRC<T>;

    type IntoIter = linked_hash_map::Values<'a, Id, RRC<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.values()
    }
}

impl<T, F> From<F> for IdList<T>
where
    T: GetName,
    F: IntoIterator<Item = RRC<T>>,
{
    fn from(list: F) -> Self {
        IdList(
            list.into_iter()
                .map(|item| {
                    let name = item.borrow().name();
                    (name, item)
                })
                .collect::<LinkedHashMap<Id, RRC<T>>>(),
        )
    }
}

impl<T: GetName> IdList<T> {
    /// Removes all elements from the collection
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns true if there are no elements in the list.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // Length of the underlying storage.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Keep only the elements in the collection which satisfy the given predicate and return the
    /// number of elements removed.
    pub fn retain<F>(&mut self, mut f: F) -> u64
    where
        F: FnMut(&RRC<T>) -> bool,
    {
        let mut removed = 0;
        for entry in self.0.entries() {
            if !f(entry.get()) {
                entry.remove();
                removed += 1;
            }
        }
        removed
    }

    /// Add a new element to the colleciton
    pub fn add(&mut self, item: RRC<T>) {
        let name = item.borrow().name();
        self.0.insert(name, item);
    }

    // Remove and return the element with the given name.
    pub fn remove<S>(&mut self, name: S) -> Option<RRC<T>>
    where
        S: Into<Id>,
    {
        self.0.remove(&name.into())
    }

    /// Add all elements to the collection
    pub fn append(&mut self, items: impl Iterator<Item = RRC<T>>) {
        let map = items.map(|i| {
            let name = i.borrow().name();
            (name, i)
        });
        self.0.extend(map);
    }

    /// Returns an iterator over immutable references
    pub fn iter(&self) -> impl Clone + Iterator<Item = &RRC<T>> {
        self.0.values()
    }

    /// Returns an iterator over mutable references. Likely a pointless method
    /// as this is a collection of RRCs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RRC<T>> {
        self.0.iter_mut().map(|(_id, val)| val)
    }

    /// Removes all elements from the collection and returns an iterator over
    /// the owned elements.
    pub fn drain(&mut self) -> impl Iterator<Item = RRC<T>> {
        let drain = std::mem::take(&mut self.0);

        drain.into_iter().map(|(_, cell)| cell)
    }

    /// Returns the element indicated by the name, if present, otherwise None.
    pub fn find<S>(&self, name: S) -> Option<RRC<T>>
    where
        S: Into<Id>,
    {
        self.0.get(&name.into()).map(Rc::clone)
    }
}

impl<T: GetName> Default for IdList<T> {
    fn default() -> Self {
        IdList(LinkedHashMap::new())
    }
}
