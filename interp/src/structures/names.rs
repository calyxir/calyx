use crate::interpreter_ir as iir;
use calyx_ir::Id;
use itertools::Itertools;
use std::{
    fmt::{Display, Write},
    hash::Hash,
    ops::Deref,
    rc::Rc,
};

#[derive(Debug, Clone)]
/// A portion of a qualified name representing an instance of a Calyx component.
pub struct InstanceName {
    /// Handle to the component definition
    pub component_id: Rc<iir::Component>,
    /// The name of the instance
    pub instance: Id,
}

impl InstanceName {
    pub fn new(component_id: &Rc<iir::Component>, instance: Id) -> Self {
        Self {
            component_id: component_id.clone(),
            instance,
        }
    }
}

impl Hash for InstanceName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (&*self.component_id as *const iir::Component).hash(state);
        self.instance.hash(state);
    }
}

impl PartialEq for InstanceName {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.component_id, &other.component_id)
            && self.instance == other.instance
    }
}

impl Eq for InstanceName {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComponentQualifiedInstanceName(Rc<Vec<InstanceName>>);

impl ComponentQualifiedInstanceName {
    pub fn as_id(&self) -> Id {
        let name = self.0.iter().map(|x| x.instance.id.as_str()).join(".");
        Id::from(name)
    }
}

impl Deref for ComponentQualifiedInstanceName {
    type Target = Vec<InstanceName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ComponentQualifiedInstanceName {
    pub fn new_extend(&self, inst: InstanceName) -> Self {
        let mut inner = (*self.0).clone();
        inner.push(inst);
        Self(Rc::new(inner))
    }

    pub fn new_single(component_id: &Rc<iir::Component>, instance: Id) -> Self {
        let inst = InstanceName::new(component_id, instance);
        Self::from(inst)
    }
}

impl<T: Into<InstanceName>> From<T> for ComponentQualifiedInstanceName {
    fn from(input: T) -> Self {
        let inst: InstanceName = input.into();
        Self(Rc::new(vec![inst]))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// The fully-qualified instance name of some calyx entity
pub struct QualifiedInstanceName {
    /// The instance names of the ancestors in the state tree
    prefix: ComponentQualifiedInstanceName,
    /// Cell/group/port name
    name: Id,
}

impl QualifiedInstanceName {
    pub fn as_id(&self) -> Id {
        let mut string_vec: Vec<String> = self
            .prefix
            .iter()
            .map(|x| x.instance.id.as_str().to_string())
            .collect();
        string_vec.push(self.name.id.as_str().to_string());
        string_vec.join(".").into()
    }

    pub fn get_suffix(&self) -> Id {
        self.name
    }

    pub fn new(prefix: &ComponentQualifiedInstanceName, name: Id) -> Self {
        Self {
            prefix: prefix.clone(),
            name,
        }
    }

    pub fn prefix_length(&self) -> usize {
        self.prefix.0.len()
    }

    pub fn shared_prefix_of(&self, other: &Self) -> bool {
        for (a, b) in self.prefix.0.iter().zip(other.prefix.0.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }

    pub fn equal_prefix(&self, other: &Self) -> bool {
        self.prefix == other.prefix
    }
}

#[derive(Debug, Clone)]
pub enum GroupName {
    /// An actual group
    Group(Id),
    /// A phantom group with a displayable name
    Phantom(Id),
    /// No group name (this allows components to be in the tree)
    None,
}

#[derive(Clone)]
pub struct GroupQualifiedInstanceName {
    pub prefix: ComponentQualifiedInstanceName,
    pub group: GroupName,
    pub pos_tag: Option<u64>,
}

impl GroupQualifiedInstanceName {
    pub fn new_group(comp: &ComponentQualifiedInstanceName, name: Id) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::Group(name),
            pos_tag: None,
        }
    }

    pub fn new_phantom(
        comp: &ComponentQualifiedInstanceName,
        name: Id,
    ) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::Phantom(name),
            pos_tag: None,
        }
    }

    pub fn new_empty(comp: &ComponentQualifiedInstanceName) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::None,
            pos_tag: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        !matches!(&self.group, GroupName::None)
    }

    pub fn has_tag(&self) -> bool {
        self.pos_tag.is_some()
    }

    pub fn format_name(&self) -> String {
        let mut out: String = self.prefix.as_id().to_string();
        match &self.group {
            GroupName::Group(g) => write!(out, "::{}", g).unwrap(),
            GroupName::Phantom(g) => write!(out, "::<{}>", g).unwrap(),
            GroupName::None => {}
        }
        out
    }

    pub fn with_tag(mut self, tag: Option<u64>) -> Self {
        self.pos_tag = tag;
        self
    }
}

impl std::fmt::Debug for GroupQualifiedInstanceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupQualifiedInstanceName")
            .field("prefix", &self.prefix.as_id())
            .field("group", &self.group)
            .field("tag", &self.pos_tag)
            .finish()
    }
}

pub type GroupQIN = QualifiedInstanceName;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CompGroupName {
    pub group_name: Id,
    pub component_name: Id,
}

impl From<GroupQIN> for CompGroupName {
    fn from(qin: GroupQIN) -> Self {
        let last = qin.prefix.last().unwrap();
        Self {
            group_name: qin.name,
            component_name: last.component_id.name,
        }
    }
}

impl CompGroupName {
    pub fn new(group_name: Id, component_name: Id) -> Self {
        Self {
            group_name,
            component_name,
        }
    }
}

impl Display for CompGroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.component_name, self.group_name)
    }
}
