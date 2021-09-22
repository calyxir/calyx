//! Inteprets a component.

use super::interpret_control::interpret_control;
use super::interpret_group::interp_cont;
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;
use calyx::ir;
//use std::cell::RefCell;
//use std::rc::Rc;

/// Interpret a component.

pub fn interpret_component<'outer>(
    comp: &ir::Component,
    env: InterpreterState<'outer>,
) -> InterpreterResult<InterpreterState<'outer>> {
    let ctrl: &ir::Control = &comp.control.borrow();
    if super::utils::control_is_empty(ctrl) {
        interp_cont(&comp.continuous_assignments, env, comp)
    } else {
        interpret_control(
            &comp.control.borrow(),
            &comp.continuous_assignments,
            env,
            comp,
        )
    }
}
