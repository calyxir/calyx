use super::super::utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::{InterpreterState, StateView};
use crate::errors::InterpreterResult;
use crate::primitives::Primitive;
use crate::utils::AsRaw;
use calyx::ir::{self, Component, Port, RRC};

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
    input_ports: Vec<RRC<Port>>,
    output_ports: Vec<RRC<Port>>,
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

        let (mut inputs, mut outputs) = (Vec::new(), Vec::new());

        for port in comp.signature.borrow().ports.iter() {
            let pt_ref = port.borrow();
            match &pt_ref.direction {
                ir::Direction::Input => outputs.push(port.clone()),
                ir::Direction::Output => inputs.push(port.clone()),
                ir::Direction::Inout => {
                    panic!()
                    // empty for now also probably shouldn't happen
                }
            }
        }

        Self {
            interp,
            input_ports: inputs,
            output_ports: outputs,
        }
    }

    fn look_up_outputs(&self) -> Vec<(ir::Id, crate::values::Value)> {
        let env = self.get_env();
        todo!()
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

    fn get_env(&self) -> Box<dyn StateView<'outer> + '_> {
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

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn super::AssignmentInterpreterMarker> {
        None
    }
}

impl<'a, 'outer> Primitive for ComponentInterpreter<'a, 'outer> {
    fn do_tick(&mut self) -> Vec<(ir::Id, crate::values::Value)> {
        self.step().expect("Error when stepping");
        self.look_up_outputs()
    }

    fn is_comb(&self) -> bool {
        false
    }

    fn validate(&self, inputs: &[(ir::Id, &crate::values::Value)]) {
        for (name, value) in inputs {
            let port = self
                .input_ports
                .iter()
                .find(|x| x.borrow().name == name)
                .expect("Component given non-existant input");
            assert_eq!(port.borrow().width, value.width())
        }
    }

    fn execute(
        &mut self,
        inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        if self.get_current_interp().is_none() {
            return vec![];
        }
        let input_vec = inputs
            .iter()
            .map(|(name, val)| {
                let port = self
                    .input_ports
                    .iter()
                    .find(|x| x.borrow().name == name)
                    .unwrap();
                (port.as_raw(), (*val).clone())
            })
            .collect::<Vec<_>>();

        let marker = self.get_current_interp().unwrap();

        for (port, value) in input_vec {
            marker.insert(port, value);
        }
        marker.step_convergence().unwrap();
        self.look_up_outputs()
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        todo!()
    }
}
