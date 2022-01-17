use crate::interpreter_ir as iir;
use calyx::ir::Id;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
/// A portion of a qualified name representing an instance of a Calyx component.
pub struct InstanceName {
    /// Handle to the component definition
    component_id: Rc<iir::Component>,
    /// The name of the instance
    instance: Id,
}

impl InstanceName {
    pub fn new(component_id: &Rc<iir::Component>, instance: &Id) -> Self {
        Self {
            component_id: component_id.clone(),
            instance: instance.clone(),
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
        let string_vec: Vec<String> =
            self.0.iter().map(|x| x.instance.clone().id).collect();
        string_vec.join(".").into()
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

    pub fn new_single(
        component_id: &Rc<iir::Component>,
        instance: &Id,
    ) -> Self {
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
        let mut string_vec: Vec<String> =
            self.prefix.iter().map(|x| x.instance.clone().id).collect();
        string_vec.push(self.name.id.clone());
        string_vec.join(".").into()
    }

    pub fn new(prefix: &ComponentQualifiedInstanceName, name: &Id) -> Self {
        Self {
            prefix: prefix.clone(),
            name: name.clone(),
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

pub enum GroupName {
    /// An actual group
    Group(Id),
    /// A phantom group with a displayable name
    Phantom(String),
    /// No group name
    None,
}

pub struct GroupQualifiedInstanceName {
    pub prefix: ComponentQualifiedInstanceName,
    pub group: GroupName,
}

impl GroupQualifiedInstanceName {
    pub fn new_group(comp: &ComponentQualifiedInstanceName, name: &Id) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::Group(name.clone()),
        }
    }

    pub fn new_phantom(
        comp: &ComponentQualifiedInstanceName,
        name: &String,
    ) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::Phantom(name.clone()),
        }
    }

    pub fn new_empty(comp: &ComponentQualifiedInstanceName) -> Self {
        Self {
            prefix: comp.clone(),
            group: GroupName::None,
        }
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
            component_name: last.component_id.name.clone(),
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
