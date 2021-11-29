use crate::interpreter_ir as iir;
use calyx::ir::Id;
use std::fmt::Display;
use std::fmt::Write;
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
pub struct ComponentQIN(Rc<Vec<InstanceName>>);

impl ComponentQIN {
    pub fn as_id(&self) -> Id {
        let mut acc = String::new();
        match self.0.len() {
            0 => {}
            _n => {
                write!(acc, "{}", &self.0[0].instance)
                    .expect("error with name?");
                for i in self.0.iter().skip(1) {
                    write!(acc, ".{}", &i.instance).expect("error with name?");
                }
            }
        }
        acc.into()
    }
}

impl Deref for ComponentQIN {
    type Target = Vec<InstanceName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ComponentQIN {
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

impl<T: Into<InstanceName>> From<T> for ComponentQIN {
    fn from(input: T) -> Self {
        let inst: InstanceName = input.into();
        Self(Rc::new(vec![inst]))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// The fully-qualified instance name of some calyx entity
pub struct QualifiedInstanceName {
    /// The instance names of the ancestors in the state tree
    prefix: ComponentQIN,
    /// Cell name
    name: Id,
}

impl QualifiedInstanceName {
    pub fn new(prefix: &ComponentQIN, name: &Id) -> Self {
        Self {
            prefix: prefix.clone(),
            name: name.clone(),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// A qualified name which does not contain instance information
pub struct QualifiedName {
    prefix: Vec<Id>,
    name: Id,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// A qualified instance group name
pub struct GroupQIN(QualifiedInstanceName);

impl GroupQIN {
    pub fn new(prefix: &ComponentQIN, name: &Id) -> Self {
        Self(QualifiedInstanceName::new(prefix, name))
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// A qualified group name
pub struct GroupQN(QualifiedName);

impl From<QualifiedInstanceName> for QualifiedName {
    fn from(qin: QualifiedInstanceName) -> Self {
        Self {
            prefix: qin
                .prefix
                .iter()
                .map(|x| x.component_id.name.clone())
                .collect(),
            name: qin.name,
        }
    }
}

impl From<QualifiedInstanceName> for GroupQIN {
    fn from(qin: QualifiedInstanceName) -> Self {
        Self(qin)
    }
}

impl From<QualifiedName> for GroupQN {
    fn from(qn: QualifiedName) -> Self {
        Self(qn)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CompGroupName {
    pub group_name: Id,
    pub component_name: Id,
}

impl From<GroupQIN> for CompGroupName {
    fn from(qin: GroupQIN) -> Self {
        let last = qin.0.prefix.last().unwrap();
        Self {
            group_name: qin.0.name,
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
