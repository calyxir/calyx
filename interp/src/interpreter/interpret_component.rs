//! Inteprets a component.

use super::interpret_control::interpret_control;
use super::interpret_group::interp_cont;
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;
use crate::interpreter_ir as iir;
//use std::cell::RefCell;
use std::rc::Rc;

/// Interpret a component.

pub fn interpret_component(
    comp: &Rc<iir::Component>,
    env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    if super::utils::control_is_empty(&comp.control) {
        interp_cont(&comp.continuous_assignments, env, comp)
    } else {
        interpret_control(
            &comp.control,
            &comp.continuous_assignments,
            env,
            &comp,
        )
    }
}
