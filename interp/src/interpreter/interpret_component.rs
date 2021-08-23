//! Inteprets a component.

use super::interpret_control::interpret_control;
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
    interpret_control(
        &comp.control.borrow(),
        &comp.continuous_assignments,
        env,
        comp,
    )
}
