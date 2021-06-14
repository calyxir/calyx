//! Inteprets a component.

use super::interpret_control::interpret_control;
use crate::environment::Environment;
use calyx::{errors::FutilResult, ir};
//use std::cell::RefCell;
//use std::rc::Rc;

/// Interpret a component.

pub fn interpret_component(
    comp: &ir::Component,
    env: Environment,
) -> FutilResult<Environment> {
    interpret_control(&comp.control.borrow(), env)
}
