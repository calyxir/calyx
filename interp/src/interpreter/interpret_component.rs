//! Inteprets a component.

use super::interpret_control::interpret_control;
use super::interpret_group::interp_cont;
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;
use calyx::ir;
//use std::cell::RefCell;
//use std::rc::Rc;

/// Interpret a component.

pub fn interpret_component(
    comp: &ir::Component,
    env: InterpreterState,
) -> InterpreterResult<InterpreterState> {
    let ctrl: &ir::Control = &comp.control.borrow();
    if let ir::Control::Empty(_) = ctrl {
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
