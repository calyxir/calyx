use std::collections::HashSet;

use super::super::utils::control_is_empty;
use super::control_interpreter::{
    ControlInterpreter, Interpreter, StructuralInterpreter,
};
use crate::environment::{InterpreterState, StateView};
use crate::errors::InterpreterResult;
use crate::primitives::Primitive;
use crate::utils::AsRaw;
use crate::values::Value;
use calyx::ir::{self, Component, Port, RRC};
use std::rc::Rc;

enum StructuralOrControl<'a, 'outer> {
    Structural(StructuralInterpreter<'a, 'outer>),
    Control(ControlInterpreter<'a, 'outer>),
    Nothing, // a default variant which is only ever around transiently
}
impl<'a, 'outer> Default for StructuralOrControl<'a, 'outer> {
    fn default() -> Self {
        Self::Nothing
    }
}

impl<'a, 'outer> StructuralOrControl<'a, 'outer> {
    fn _is_structural(&self) -> bool {
        matches!(self, Self::Structural(_))
    }

    fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }
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
    comp_ref: &'a Component,
    control_ref: &'a ir::Control,
    done_port: RRC<Port>,
    go_port: RRC<Port>,
    input_hash_set: Rc<HashSet<*const ir::Port>>,
}

impl<'a, 'outer> ComponentInterpreter<'a, 'outer> {
    pub fn from_component(
        comp: &'a Component,
        control: &'a ir::Control,
        env: InterpreterState<'outer>,
    ) -> Self {
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

        let input_hash_set =
            inputs.iter().map(|x| x.as_raw()).collect::<HashSet<_>>();

        let input_hash_set = Rc::new(input_hash_set);
        let interp;

        if control_is_empty(control) {
            interp = StructuralInterpreter::from_component(comp, env).into();
        } else {
            interp = ControlInterpreter::new(
                control,
                env,
                &comp.continuous_assignments,
                Rc::clone(&input_hash_set),
            )
            .into()
        };

        let go_port = inputs
            .iter()
            .find(|x| x.borrow().attributes.has("go"))
            .unwrap()
            .clone();
        let done_port = outputs
            .iter()
            .find(|x| x.borrow().attributes.has("done"))
            .unwrap()
            .clone();

        Self {
            interp,
            input_ports: inputs,
            output_ports: outputs,
            comp_ref: comp,
            control_ref: control,
            go_port,
            done_port,
            input_hash_set,
        }
    }

    fn look_up_outputs(&self) -> Vec<(ir::Id, crate::values::Value)> {
        let env = self.get_env();
        self.output_ports
            .iter()
            .map(|x| {
                let port_ref = x.borrow();
                (port_ref.name.clone(), env.lookup(x.as_raw()).clone())
            })
            .collect()
    }

    fn go_is_high(&self) -> bool {
        self.get_env().lookup(self.go_port.as_raw()).as_u64() == 1
    }

    fn done_is_high(&self) -> bool {
        self.get_env().lookup(self.done_port.as_raw()).as_u64() == 1
    }

    pub fn set_go_high(&mut self) {
        let raw = self.go_port.as_raw();
        self.get_mut_env().insert(raw, Value::bit_high())
    }

    pub fn set_go_low(&mut self) {
        let raw = self.go_port.as_raw();
        self.get_mut_env().insert(raw, Value::bit_low())
    }

    fn set_done_high(&mut self) {
        let raw = self.done_port.as_raw();
        self.get_mut_env().insert(raw, Value::bit_high())
    }

    fn set_done_low(&mut self) {
        let raw = self.done_port.as_raw();
        self.get_mut_env().insert(raw, Value::bit_low())
    }
}

impl<'a, 'outer> Interpreter<'outer> for ComponentInterpreter<'a, 'outer> {
    fn step(&mut self) -> InterpreterResult<()> {
        let go = self.go_is_high();
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.step(),
            StructuralOrControl::Control(c) => {
                if go {
                    c.step()
                } else {
                    Ok(())
                }
            }
            _ => unreachable!(""),
        }
    }

    fn deconstruct(self) -> InterpreterState<'outer> {
        match self.interp {
            StructuralOrControl::Structural(s) => s.deconstruct(),
            StructuralOrControl::Control(c) => c.deconstruct(),
            _ => unreachable!(""),
        }
    }

    fn is_done(&self) -> bool {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.is_done(),
            StructuralOrControl::Control(c) => c.is_done(),
            _ => unreachable!(""),
        }
    }

    fn get_env(&self) -> StateView<'_, 'outer> {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.get_env(),
            StructuralOrControl::Control(c) => c.get_env(),
            _ => unreachable!(""),
        }
    }

    fn currently_executing_group(&self) -> Vec<&ir::Id> {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.currently_executing_group(),
            StructuralOrControl::Control(c) => c.currently_executing_group(),
            _ => unreachable!(""),
        }
    }

    fn get_current_interp(
        &mut self,
    ) -> Option<&mut dyn super::AssignmentInterpreterMarker> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.get_current_interp(),
            StructuralOrControl::Control(c) => c.get_current_interp(),
            StructuralOrControl::Nothing => unreachable!(),
        }
    }

    fn get_mut_env(&mut self) -> crate::environment::MutStateView<'_, 'outer> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.get_mut_env(),
            StructuralOrControl::Control(c) => c.get_mut_env(),
            StructuralOrControl::Nothing => unreachable!(),
        }
    }

    fn converge(&mut self) -> InterpreterResult<()> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.converge(),
            StructuralOrControl::Control(c) => c.converge(),
            StructuralOrControl::Nothing => unreachable!(),
        }
    }
}

impl<'a, 'outer> Primitive for ComponentInterpreter<'a, 'outer> {
    fn do_tick(&mut self) -> Vec<(ir::Id, Value)> {
        let currently_done = self.done_is_high();

        // this component has been done for a cycle
        if currently_done {
            self.reset(&[]);
        } else {
            self.step().expect("Error when stepping");
        }

        // just became done for an imperative component
        // so set done high
        if !currently_done && self.is_done() && self.interp.is_control() {
            self.set_done_high()
        }

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

        let mut env = self.get_mut_env();

        for (port, value) in input_vec {
            env.insert(port, value);
        }
        self.converge().unwrap();

        self.look_up_outputs()
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &crate::values::Value)],
    ) -> Vec<(ir::Id, crate::values::Value)> {
        assert!(
            self.is_done(),
            "Component interpreter reset before finishing"
        );

        let interp = std::mem::take(&mut self.interp);

        let new = match interp {
            StructuralOrControl::Structural(s) => {
                StructuralOrControl::Structural(s)
            }
            StructuralOrControl::Control(control) => {
                let env = control.deconstruct();
                ControlInterpreter::new(
                    self.control_ref,
                    env,
                    &self.comp_ref.continuous_assignments,
                    self.input_hash_set.clone(),
                )
                .into()
            }
            StructuralOrControl::Nothing => unreachable!(),
        };

        self.interp = new;
        self.set_done_low();

        self.look_up_outputs()
    }
}

// pub trait ComponentInterpreterMarker {
//     fn set_go_low(&mut self);
//     fn set_go_high(&mut self);
// }

// impl<'a, 'outer> ComponentInterpreterMarker
//     for ComponentInterpreter<'a, 'outer>
// {
//     fn set_go_low(&mut self) {
//         self.set_done_low();
//     }

//     fn set_go_high(&mut self) {
//         self.set_go_high()
//     }
// }
