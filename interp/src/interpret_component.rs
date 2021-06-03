//! Inteprets a component.

use super::{environment::Environment, interpret_control::ControlInterpreter};
use calyx::{errors::FutilResult, ir};
//use std::cell::RefCell;
//use std::rc::Rc;

/// Interpret a component.
pub struct ComponentInterpreter {
    /// The environment
    pub environment: Environment,

    /// The component itself
    /// For example, when interpreting a multi-component program, we first interpret main.
    /// We would then interpret other components as necessary.
    /// TODO: probably want a reference to context instead, so that we can clone Environments?
    pub component: ir::Component,
}

impl ComponentInterpreter {
    /// Construct ComponentInterpreter
    /// env : Initial environment
    /// comp : The component to interpret
    /// (TODO: should comp be an RRC<component> instead?)
    // pub fn init<'a>(env: Environment, comp: &'a ir::Component) -> Self {
    //     //let ok = True;
    //     Self {
    //         environment: env,
    //         component: comp,
    //     }
    // }

    /// Interpret this component.
    /// TODO: currently moves the control into the Control interpreter
    pub fn interpret(self) -> FutilResult<Environment> {
        let ci: ControlInterpreter = ControlInterpreter::init(
            self.environment,
            self.component.name.id.clone(),
            self.component.control,
        );
        ci.interpret()
    }
}
