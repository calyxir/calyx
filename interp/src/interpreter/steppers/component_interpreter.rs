use super::super::utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::InterpreterState;
use calyx::ir::{self, Component};

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
    interp: StructuralOrControl<'a>,
}

impl<'a> ComponentInterpreter<'a> {
    pub fn from_component(
        comp: &'a Component,
        control: &'a ir::Control,
        env: InterpreterState,
    ) -> Self {
        let interp;

        if control_is_empty(&control) {
            interp = StructuralInterpreter::from_component(comp, env).into();
        } else {
            interp = ControlInterpreter::new(
                control,
                env,
                &comp.continuous_assignments,
            )
            .into()
        };

        Self { interp }
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
