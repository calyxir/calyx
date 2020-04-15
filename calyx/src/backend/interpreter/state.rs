use crate::errors::Error;
use crate::lang::ast;
use crate::lang::component::Component;
use crate::lang::context::Context;
use crate::lang::structure::NodeData;
use std::collections::HashMap;

/// Recursively stores a component's state
#[derive(Debug, Clone)]
pub enum State {
    /// Mapping from instance Ids to states for user-defined subcomponents
    Component(HashMap<ast::Id, State>),
    /// State for a primitive register component
    Register(Option<i64>),
    Empty,
}

impl State {
    pub fn from_component(
        comp: &Component,
        c: &Context,
    ) -> Result<Self, Error> {
        // Check if the component is a primitive
        if c.is_lib(&comp.name) {
            if comp.name.to_string() == "std_reg" {
                return Ok(State::Register(None));
            } else {
                return Ok(State::Empty);
            }
        }

        let mut map = HashMap::new();
        for (_idx, data) in comp.structure.instances() {
            match data {
                NodeData::Input(_) => { /* do nothing */ }
                NodeData::Output(_) => { /* do nothing */ }
                NodeData::Instance {
                    name,
                    component_type,
                    ..
                } => {
                    if c.is_lib(&name)
                        && component_type.to_string() == "std_reg"
                    {
                        map.insert(name, State::Register(None));
                    } else if !c.is_lib(&name) {
                        map.insert(
                            name,
                            State::from_component(
                                &c.get_component(&component_type)?,
                                c,
                            )?,
                        );
                    }
                }
            }
        }
        Ok(State::Component(map))
    }
}
