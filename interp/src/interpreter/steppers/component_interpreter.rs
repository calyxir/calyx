use super::super::utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::InterpreterState;
use crate::errors::InterpreterResult;
use crate::primitives::Primitive;
use calyx::ir::{self, Component};

enum StructuralOrControl<'a, 'outer> {
    Structural(StructuralInterpreter<'a, 'outer>),
    Control(ControlInterpreter<'a, 'outer>),
}

impl<'a, 'outer> From<StructuralInterpreter<'a, 'outer>>
    for StructuralOrControl<'a, 'outer>
{
    fn from(input: StructuralInterpreter<'a, 'outer>) -> Self {
        Self::Structural(input)
    }
}

impl<'a, 'outer> From<ControlInterpreter<'a, 'outer>>
    for StructuralOrControl<'a, 'outer>
{
    fn from(input: ControlInterpreter<'a, 'outer>) -> Self {
        Self::Control(input)
    }
}

pub struct ComponentInterpreter<'a, 'outer> {
    interp: StructuralOrControl<'a, 'outer>,
}

impl<'a, 'outer> ComponentInterpreter<'a, 'outer> {
    pub fn from_component(
        comp: &'a Component,
        control: &'a ir::Control,
        env: InterpreterState<'outer>,
    ) -> Self {
        let interp;

        if control_is_empty(control) {
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

impl<'a, 'outer> Interpreter<'outer> for ComponentInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.step(),
            StructuralOrControl::Control(c) => c.step(),
        }
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
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

    fn get_env(&self) -> Vec<&InterpreterState<'outer>> {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.get_env(),
            StructuralOrControl::Control(c) => c.get_env(),
        }
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.currently_executing_group(),
            StructuralOrControl::Control(c) => c.currently_executing_group(),
        }
    }
}

impl<'a, 'outer> Primitive for ComponentInterpreter<'a, 'outer> {
    fn do_tick(&mut self) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }

    fn is_comb(&self) -> bool {
        todo!()
    }

    fn validate(&self, _inputs: &[(ir::Id, &crate::values::Value)]) {
        todo!()
    }

    fn execute(
        &mut self,
        _inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }
}
