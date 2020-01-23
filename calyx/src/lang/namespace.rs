use crate::lang::ast::{ComponentDef, NamespaceDef};
use std::collections::HashMap;

#[allow(unused)]
impl NamespaceDef {
    pub fn get_component(&self, component: String) -> ComponentDef {
        for c in self.components.iter() {
            if c.name == component {
                return c.clone();
            }
        }
        panic!(
            "Component \"{}\" not found in Namespace \"{}\"!",
            component, self.name
        );
    }

    pub fn get_definitions(&self) -> HashMap<String, ComponentDef> {
        let mut defs: HashMap<String, ComponentDef> = HashMap::new();
        for c in self.components.iter() {
            defs.insert(c.name.clone(), c.clone());
        }
        defs
    }
}
