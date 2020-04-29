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
    /// State::Register type
    pub fn set_reg(self, new_val: Option<i64>) -> Result<Self, Error> {
        match self {
            State::Register(_) => {
                let new_reg = State::Register(new_val);
                Ok(new_reg)
            }
            _ => Err(Error::MissingState),
        }
    }

    /// Looks up a register value in a component.
    /// This function must be called from a
    /// State::Register type
    pub fn lookup_reg(&self) -> Result<Option<i64>, Error> {
        match self {
            State::Register(val) => Ok(val.clone()),
            _ => Err(Error::MissingState),
        }
    }

    pub fn lookup_subcomp_st(&self, id: &ast::Id) -> Result<State, Error> {
        match self {
            // TODO remove unwrap
            State::Component(map) => Ok(map.get(id).unwrap().clone()),
            _ => Err(Error::InternalInterpreterError(
                "Unable to lookup sub component state in state.rs!".to_owned(),
            )),
        }
    }

    pub fn set_subcomp_st(
        &mut self,
        id: &ast::Id,
        st: State,
    ) -> Result<(), Error> {
        match self {
            // TODO remove unwrap
            State::Component(map) => {
                map.insert(id.clone(), st);
                Ok(())
            }

            _ => Err(Error::InternalInterpreterError(
                "Unable to set sub component state in state.rs!".to_owned(),
            )),
        }
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
                    println!(
                        "Generating instance State: {:#?} {:#?}",
                        name, component_type
                    );
                    if c.is_lib(&component_type)
                        && component_type.to_string() == "std_reg"
                    {
                        map.insert(name, State::Register(None));
                    } else if !c.is_lib(&component_type) {
                        map.insert(
                            name,
                            State::from_component(
                                &c.get_component(&component_type)?,
                                c,
                            )?,
                        );
                    } else {
                        map.insert(name, State::Empty);
                    }
                }
            }
        }
        Ok(State::Component(map))
    }
}
