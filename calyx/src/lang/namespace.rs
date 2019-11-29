use crate::lang::ast::{Component, Namespace};
use std::collections::HashMap;

#[allow(unused)]
impl Namespace {
    pub fn get_component(&self, component: String) -> Component {
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

    pub fn get_definitions(&self) -> HashMap<String, Component> {
        let mut defs: HashMap<String, Component> = HashMap::new();
        for c in self.components.iter() {
            defs.insert(c.name.clone(), c.clone());
        }
        defs
    }
}
