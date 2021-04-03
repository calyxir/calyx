//! Inteprets a component.

use super::{environment::Environment, interpret_control::ControlInterpreter};
use calyx::{
    errors::{Error, FutilResult},
    ir,
};
use std::rc::Rc;

/// Interpret a component.
pub struct ComponentInterpreter {
    /// The environment
    pub environment: Environment,

    /// The component itself
    /// For example, when interpreting a multi-component program,
    /// we will first interpret main.
    /// We would then interpret other components as necessary.
    /// TODO: probably want a reference to context instead, so that we can clone Environments?
    pub component: ir::Component,
}

impl ComponentInterpreter {
    /// Interpret this component.
    /// TODO: currently moves the control into the Control interpreter
    pub fn interpret(self) -> FutilResult<Environment> {
        let ci: ControlInterpreter = ControlInterpreter {
            environment: self.environment,
            component: self.component.name.id.clone(),
            control: self.component.control,
        };
        ci.interpret()
    }
}
