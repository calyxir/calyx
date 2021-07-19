use super::super::simulation_utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::InterpreterState;
use crate::values::Value;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use calyx::ir::{self, Component};
use std::cell::Ref;

enum StructuralOrControl<'a> {
    Structural(StructuralInterpreter<'a>),
    Control(ControlInterpreter<'a>),
}

impl<'a> From<StructuralInterpreter<'a>> for StructuralOrControl<'a> {
    fn from(input: StructuralInterpreter<'a>) -> Self {
        Self::Structural(input)
    }
}

impl<'a> From<ControlInterpreter<'a>> for StructuralOrControl<'a> {
    fn from(input: ControlInterpreter<'a>) -> Self {
        Self::Control(input)
    }
}

impl<'a> Deref for StructuralOrControl<'a> {
    type Target = dyn Interpreter + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            StructuralOrControl::Structural(inner) => inner,
            StructuralOrControl::Control(inner) => inner,
        }
    }
}

impl<'a> DerefMut for StructuralOrControl<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            StructuralOrControl::Structural(inner) => inner,
            StructuralOrControl::Control(inner) => inner,
        }
    }
}

pub struct ComponentInterpreter<'a> {
    // in_ports: HashMap<ir::Id, Value>,
    comp: &'a Component,
    control: Ref<'a, ir::Control>, // keep a ref to the control struct alive
    interp: StructuralOrControl<'a>,
}

impl<'a> ComponentInterpreter<'a> {
    pub fn from_component(comp: &'a Component, env: InterpreterState) -> Self {
        let interp;
        let control = comp.control.borrow();
        if control_is_empty(&control) {
            interp = StructuralInterpreter::from_component(comp, env).into();
        } else {
            interp = ControlInterpreter::new(
                &control,
                env,
                &comp.continuous_assignments,
            )
            .into()
        };

        Self {
            comp,
            control,
            interp,
        }
    }
}
