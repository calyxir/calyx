//! Inteprets a component.

use super::interpret_control::interpret_control;
use crate::environment::InterpreterState;
use calyx::{errors::FutilResult, ir};
//use std::cell::RefCell;
//use std::rc::Rc;

/// Interpret a component.

pub fn interpret_component(
    comp: &ir::Component,
    env: InterpreterState,
) -> FutilResult<InterpreterState> {
    interpret_control(
        &comp.control.borrow(),
        &comp.continuous_assignments,
        env,
        comp,
    )
}
