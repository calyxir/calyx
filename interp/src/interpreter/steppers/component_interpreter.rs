use super::super::utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::InterpreterState;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::primitives::Primitive;
use calyx::ir::{self, Cell, Component, Context, Port, RRC};

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

    pub fn new(ctx: &ir::Context, cell: &RRC<Cell>) -> Self {
        let cell_borrow = cell.borrow();
        if let ir::CellType::Component { name: comp_name } =
            &cell_borrow.prototype
        {
            // If there is no component with this name then the parsing into IR should
            // break
            let component =
                ctx.components.iter().find(|x| x.name == comp_name).unwrap();
        } else {
            // If this happens it's definitely an error in the interpreter code
            panic!("New component called on something that is not a component")
        }

        todo!()
    }
}

impl<'a> Interpreter for ComponentInterpreter<'a> {
    fn step(&mut self) -> InterpreterResult<()> {
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

    fn get_env(&self) -> Vec<&InterpreterState> {
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

impl<'a> Primitive for ComponentInterpreter<'a> {
    fn do_tick(&mut self) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }

    fn is_comb(&self) -> bool {
        todo!()
    }

    fn validate(&self, inputs: &[(ir::Id, &crate::values::Value)]) {
        todo!()
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }

    fn reset(
        &mut self,
        inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }
}
