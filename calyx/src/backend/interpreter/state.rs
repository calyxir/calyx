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
    /// Sets a register value in a component.
    /// This function must be called from a
    /// State::Component type, which are generated
    /// automatically by `from_component`, and the
    /// component with `id` must be a `std_reg`
    pub fn set_reg(
        self,
        id: &ast::Id,
        new_val: Option<i64>,
    ) -> Result<Self, Error> {
        // match self {
        //     State::Component(mut map) => {
        //         if let Some(next_state) = map.get(id) {
        match self {
            State::Register(_) => {
                let new_reg = State::Register(new_val);
                Ok(new_reg)
            }
            _ => Err(Error::MissingState),
        }
        //         } else {
        //             Err(Error::MissingState)
        //         }
        //     }
        //     _ => Err(Error::MissingState),
        // }
    }

    /// Looks up a register value in a component.
    /// This function must be called from a
    /// State::Component type, which are generated
    /// automatically by `from_component`, and the
    /// component with `id` must be a `std_reg`
    pub fn lookup_reg(&self, id: &ast::Id) -> Result<Option<i64>, Error> {
        // match self {
        //     State::Component(map) => {
        //         if let Some(next_state) = map.get(id) {
        match self {
            State::Register(val) => Ok(val.clone()),
            _ => Err(Error::MissingState),
        }
        //         } else {
        //             Err(Error::MissingState)
        //         }
        //     }
        //     _ => Err(Error::MissingState),
        // }
    }

    /// Generates a state for a component
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
