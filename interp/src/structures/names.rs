use crate::interpreter_ir as iir;
use calyx::ir::Id;
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

/// The fully-qualified instance name of some calyx entity
pub struct QualifiedInstanceName {
    /// The instance names of the ancestors in the state tree
    prefix: Vec<InstanceName>,
    /// Cell name
    name: Id,
}

/// A qualified name which does not contain instance information
pub struct QualifiedName {
    prefix: Vec<Id>,
    name: Id,
}

/// A qualified instance group name
pub struct GroupQIN(QualifiedInstanceName);

/// A qualified group name
pub struct GroupQN(QualifiedName);

impl From<QualifiedInstanceName> for QualifiedName {
    fn from(qin: QualifiedInstanceName) -> Self {
        Self {
            prefix: qin
                .prefix
                .into_iter()
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
