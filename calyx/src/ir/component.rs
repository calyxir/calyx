use super::{
    Assignment, Attributes, Builder, Cell, CellType, CloneName, Control,
    Direction, GetName, Group, Id, RRC,
};
use crate::utils;
use linked_hash_map::LinkedHashMap;
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
    pub cells: IdList<Cell>,
    /// Groups of assignment wires.
    pub groups: IdList<Group>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Vec<Assignment>,
    /// The control program for this component.
    pub control: RRC<Control>,
    /// Attributes for this component
    pub attributes: Attributes,

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
    pub fn new<S, N>(
        name: S,
        ports: Vec<(N, u64, Direction, Attributes)>,
    ) -> Self
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
                .map(|(name, w, d, attrs)| {
                    (name.as_ref().into(), w, d.reverse(), attrs)
                })
                .collect(),
        );

        Component {
            name: name.as_ref().into(),
            signature: this_sig,
            cells: IdList::default(),
            groups: IdList::default(),
            continuous_assignments: vec![],
            control: Rc::new(RefCell::new(Control::empty())),
            namegen: utils::NameGenerator::default(),
            attributes: Attributes::default(),
        }
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_group<S>(&self, name: &S) -> Option<RRC<Group>>
    where
        S: Clone + AsRef<str>,
    {
        self.groups.find(name)
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell<S>(&self, name: &S) -> Option<RRC<Cell>>
    where
        S: Clone + AsRef<str>,
    {
        self.cells.find(name)
    }

    /// Construct a non-conflicting name using the Component's namegenerator.
    pub fn generate_name<S>(&mut self, prefix: S) -> Id
    where
        S: Into<Id> + ToString,
    {
        self.namegen.gen_name(prefix)
    }
}

#[derive(Debug)]
pub struct IdList<T: GetName>(LinkedHashMap<Id, RRC<T>>);

impl<T: GetName> IdList<T> {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&RRC<T>) -> bool,
    {
        for entry in self.0.entries() {
            if !f(entry.get()) {
                entry.remove();
            }
        }
    }

    pub fn add(&mut self, item: RRC<T>) {
        let name = item.clone_name();
        self.0.insert(name, item);
    }

    pub fn add_multiple(&mut self, items: Vec<RRC<T>>) {
        for item in items {
            self.add(item)
        }
    }

    pub fn add_multiple_by_ref(&mut self, items: &[RRC<T>]) {
        for item in items {
            self.add(item.clone())
        }
    }

    pub fn iter(&self) -> impl Clone + Iterator<Item = &RRC<T>> {
        self.0.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RRC<T>> {
        self.0.iter_mut().map(|(_id, val)| val)
    }

    pub fn drain(&mut self) -> impl Iterator<Item = RRC<T>> {
        let drain = std::mem::take(&mut self.0);

        drain.into_iter().map(|(_, cell)| cell)
    }

    pub fn find<S>(&self, name: &S) -> Option<RRC<T>>
    where
        S: Clone + AsRef<str>,
    {
        self.0.get(&name.as_ref().into()).map(|r| Rc::clone(r))
    }
}

impl<T: GetName> Default for IdList<T> {
    fn default() -> Self {
        IdList(LinkedHashMap::new())
    }
}
