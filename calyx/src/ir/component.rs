use super::{
    Assignment, Attributes, Builder, Cell, CellType, CloneName, CombGroup,
    Control, GetName, Group, Id, PortDef, RRC,
};
use crate::utils;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::cell::RefCell;
use std::collections::HashSet;
use std::iter::Extend;
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
    pub cells: IdList<Cell>,
    /// Groups of assignment wires.
    pub groups: IdList<Group>,
    /// Groups of assignment wires.
    pub comb_groups: IdList<CombGroup>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Vec<Assignment>,
    /// The control program for this component.
    pub control: RRC<Control>,
    /// Attributes for this component
    pub attributes: Attributes,
    /// True iff component is combinational
    pub is_comb: bool,

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
    pub fn new<S>(name: S, ports: Vec<PortDef<u64>>, is_comb: bool) -> Self
    where
        S: Into<Id>,
    {
        let prev_names: HashSet<_> = ports.iter().map(|pd| pd.name).collect();

        let this_sig = Builder::cell_from_signature(
            THIS_ID.into(),
            CellType::ThisComponent,
            ports
                .into_iter()
                // Reverse the port directions inside the component.
                .map(|pd| PortDef {
                    direction: pd.direction.reverse(),
                    ..pd
                })
                .collect(),
        );

        Component {
            name: name.into(),
            signature: this_sig,
            cells: IdList::default(),
            groups: IdList::default(),
            comb_groups: IdList::default(),
            continuous_assignments: vec![],
            control: Rc::new(RefCell::new(Control::empty())),
            namegen: utils::NameGenerator::with_prev_defined_names(prev_names),
            attributes: Attributes::default(),
            is_comb,
        }
    }

    pub(super) fn add_names(&mut self, names: HashSet<Id>) {
        self.namegen.add_names(names)
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_group<S>(&self, name: S) -> Option<RRC<Group>>
    where
        S: Into<Id>,
    {
        self.groups.find(name)
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

    /// Construct a non-conflicting name using the Component's namegenerator.
    pub fn generate_name<S>(&mut self, prefix: S) -> Id
    where
        S: Into<Id>,
    {
        self.namegen.gen_name(prefix)
    }

    /// Apply function on all assignments contained within the component.
    pub fn for_each_assignment<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Assignment),
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
}

/// A wrapper struct exposing an ordered collection of named entities within an
/// RRC with deterministic iteration and constant-time look-up on names
/// directly. The struct assumes that the name of an entity cannot change. Doing
/// so will introduce incorrect results for look-ups.
#[derive(Debug)]
pub struct IdList<T: GetName>(LinkedHashMap<Id, RRC<T>>);

/// Simple into-iter impl delegating to the [`Values`](linked_hash_map::Values).
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
        let name = item.clone_name();
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
        let map = items.map(|i| (i.clone_name(), i));
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
