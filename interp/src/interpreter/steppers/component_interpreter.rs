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

        // SAFETY REQUIREMENTS
        // (https://doc.rust-lang.org/std/primitive.pointer.html#method.as_ref)
        // 1. The pointer is properly aligned.
        //
        // This is covered (I hope) because we are getting this pointer from the
        // refcell.
        //
        // 2. The pointer must be "dereferenceable"
        //
        //    Same as the previous reason
        //
        // 3. The pointer must point to an initialized instance of T
        //
        //    Covered as before
        //
        //
        // 4. You must enforce aliasing rules. In particular, for the duration
        //    of this lifetime, the memory pointed to must not get mutated.
        //
        //    The Ref object which is used to get this reference is kept by the
        //    component interpreter and as a result it is not possible to obtain
        //    a mutable reference to the data, nor is it possible for the data
        //    to be deallocated while the component interpreter is alive. When
        //    the component interpreter drops, then this reference will
        //    disappear as well
        let ctrl_ptr: &ir::Control = unsafe { &*comp.control.as_ptr() };

        if control_is_empty(&control) {
            interp = StructuralInterpreter::from_component(comp, env).into();
        } else {
            interp = ControlInterpreter::new(
                &ctrl_ptr,
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

impl<'a> Interpreter for ComponentInterpreter<'a> {
    fn step(&mut self) {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.step(),
            StructuralOrControl::Control(c) => c.step(),
        }
    }

    fn deconstruct(self) -> InterpreterState {
        match self.interp {
            StructuralOrControl::Structural(s) => s.deconstruct(),
            StructuralOrControl::Control(c) => c.deconstruct(),
        }
    }

    fn is_done(&self) -> bool {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.is_done(),
            StructuralOrControl::Control(c) => c.is_done(),
        }
    }
}
