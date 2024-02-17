use std::collections::HashSet;

use super::{
    control_interpreter::{
        ComponentInfo, ControlInterpreter, StructuralInterpreter,
    },
    utils::control_is_empty,
    Interpreter,
};
use crate::{
    debugger::{name_tree::ActiveTreeNode, PrintCode},
    environment::InterpreterState,
    errors::InterpreterResult,
    interpreter_ir as iir,
    primitives::{Named, Primitive},
    structures::names::{
        ComponentQualifiedInstanceName, GroupQIN, GroupQualifiedInstanceName,
    },
    structures::state_views::{MutStateView, StateView},
    utils::AsRaw,
    values::Value,
};
use calyx_ir::{self as ir, Port, RRC};
use std::rc::Rc;

enum StructuralOrControl {
    Structural(Box<StructuralInterpreter>),
    Control(ControlInterpreter),
    Nothing, // a default variant which is only ever around transiently
}
impl Default for StructuralOrControl {
    fn default() -> Self {
        Self::Nothing
    }
}

impl StructuralOrControl {
    fn _is_structural(&self) -> bool {
        matches!(self, Self::Structural(_))
    }

    fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }
}

impl From<StructuralInterpreter> for StructuralOrControl {
    fn from(input: StructuralInterpreter) -> Self {
        Self::Structural(input.into())
    }
}

impl From<ControlInterpreter> for StructuralOrControl {
    fn from(input: ControlInterpreter) -> Self {
        Self::Control(input)
    }
}

pub struct ComponentInterpreter {
    interp: StructuralOrControl,
    input_ports: Vec<RRC<Port>>,
    output_ports: Vec<RRC<Port>>,
    comp_ref: Rc<iir::Component>,
    control_ref: iir::Control,
    done_port: RRC<Port>,
    go_port: RRC<Port>,
    input_hash_set: Rc<HashSet<*const ir::Port>>,
    qual_name: ComponentQualifiedInstanceName,
    /// used to satisfy the Named requirement for primitives, primarially for error messages
    full_name_clone: ir::Id,
}

impl ComponentInterpreter {
    pub fn make_main_component(
        env: InterpreterState,
        comp: &Rc<iir::Component>,
    ) -> Self {
        let qin = ComponentQualifiedInstanceName::new_single(comp, comp.name);
        Self::from_component(comp, env, qin)
    }

    pub fn from_component(
        comp: &Rc<iir::Component>,
        env: InterpreterState,
        qin: ComponentQualifiedInstanceName,
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
        let control = comp.control.clone();

        let go_port = inputs
            .iter()
            .find(|x| x.borrow().attributes.has(ir::NumAttr::Go))
            .unwrap()
            .clone();

        let done_port = outputs
            .iter()
            .find(|x| x.borrow().attributes.has(ir::NumAttr::Done))
            .unwrap()
            .clone();

        let mut override_set =
            inputs.iter().map(|x| x.as_raw()).collect::<HashSet<_>>();
        override_set.insert(done_port.as_raw());

        // Need to include continuous assignments in the override
        for assignment in comp.continuous_assignments.iter() {
            override_set.insert(assignment.dst.as_raw());
            let dst_ref = assignment.dst.borrow();
            if let ir::PortParent::Cell(c) = &dst_ref.parent {
                let cell = c.upgrade();
                let cell_ref = cell.borrow();
                for port in cell_ref.ports() {
                    let port_ref = port.borrow();
                    if let calyx_ir::Direction::Output = port_ref.direction {
                        override_set.insert(port.as_raw());
                    }
                }
            }
        }

        let input_hash_set = Rc::new(override_set);

        let interp = if control_is_empty(&control) {
            StructuralInterpreter::from_component(comp, env).into()
        } else {
            ControlInterpreter::new(
                control.clone(),
                env,
                &ComponentInfo::new(
                    comp.continuous_assignments.clone(),
                    input_hash_set.clone(),
                    qin.clone(),
                ),
            )
            .into()
        };
        let full_clone = qin.as_id();

        Self {
            interp,
            input_ports: inputs,
            output_ports: outputs,
            comp_ref: Rc::clone(comp),
            control_ref: control,
            go_port,
            done_port,
            input_hash_set,
            qual_name: qin,
            full_name_clone: full_clone,
        }
    }

    fn look_up_outputs(&self) -> Vec<(ir::Id, crate::values::Value)> {
        let env = self.get_env();
        self.output_ports
            .iter()
            .map(|x| {
                let port_ref = x.borrow();
                (port_ref.name, env.lookup(x.as_raw()).clone())
            })
            .collect()
    }

    #[inline]
    fn go_is_high(&self) -> bool {
        self.get_env().lookup(self.go_port.as_raw()).as_bool()
    }

    #[inline]
    fn done_is_high(&self) -> bool {
        self.get_env().lookup(self.done_port.as_raw()).as_bool()
    }

    #[inline]
    pub fn set_go_high(&mut self) {
        let raw = self.go_port.as_raw();
        self.get_env_mut().insert(raw, Value::bit_high())
    }

    #[inline]
    pub fn set_go_low(&mut self) {
        let raw = self.go_port.as_raw();
        self.get_env_mut().insert(raw, Value::bit_low())
    }

    #[inline]
    fn set_done_high(&mut self) {
        let raw = self.done_port.as_raw();
        self.get_env_mut().insert(raw, Value::bit_high())
    }

    #[inline]
    fn set_done_low(&mut self) {
        let raw = self.done_port.as_raw();
        self.get_env_mut().insert(raw, Value::bit_low())
    }

    /// Interpret a calyx program from the root
    pub fn interpret_program(
        env: InterpreterState,
        comp: &Rc<iir::Component>,
    ) -> InterpreterResult<InterpreterState> {
        let qin = ComponentQualifiedInstanceName::new_single(comp, comp.name);
        let mut main_comp = Self::from_component(comp, env, qin);
        main_comp.set_go_high();
        main_comp.run()?;
        main_comp.set_go_low();
        main_comp.deconstruct()
    }
}

impl Interpreter for ComponentInterpreter {
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

            _ => unreachable!(),
        }
    }

    fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        match self.interp {
            StructuralOrControl::Structural(s) => s.deconstruct(),
            StructuralOrControl::Control(c) => c.deconstruct(),
            _ => unreachable!(),
        }
    }

    fn is_done(&self) -> bool {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.is_done(),
            StructuralOrControl::Control(c) => c.is_done(),
            _ => unreachable!(),
        }
    }

    fn get_env(&self) -> StateView<'_> {
        match &self.interp {
            StructuralOrControl::Structural(s) => s.get_env(),
            StructuralOrControl::Control(c) => c.get_env(),
            _ => unreachable!(),
        }
    }

    fn currently_executing_group(&self) -> HashSet<GroupQIN> {
        if self.go_is_high() {
            let sub_comps = self.get_env().sub_component_currently_executing();

            // merge the sets
            &sub_comps
                | &(match &self.interp {
                    StructuralOrControl::Control(c) => {
                        c.currently_executing_group()
                    }

                    StructuralOrControl::Structural(_) => HashSet::new(),

                    _ => unreachable!(),
                })
        } else {
            HashSet::new()
        }
    }

    fn get_env_mut(&mut self) -> MutStateView<'_> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.get_env_mut(),
            StructuralOrControl::Control(c) => c.get_env_mut(),
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

    fn run(&mut self) -> InterpreterResult<()> {
        match &mut self.interp {
            StructuralOrControl::Structural(s) => s.run(),
            StructuralOrControl::Control(c) => c.run(),
            StructuralOrControl::Nothing => unreachable!(),
        }
    }

    fn get_active_tree(&self) -> Vec<ActiveTreeNode> {
        if self.go_is_high() {
            let children = match &self.interp {
                // TODO (Griffin): Include structural info
                StructuralOrControl::Structural(_) => {
                    vec![]
                }
                StructuralOrControl::Control(c) => c.get_active_tree(),
                StructuralOrControl::Nothing => unreachable!(),
            };

            let env = self.get_env();

            let sub_comp_children = env.get_active_tree();

            let mut root_node = ActiveTreeNode::new(
                GroupQualifiedInstanceName::new_empty(&self.qual_name),
            );

            for x in children.into_iter().chain(sub_comp_children.into_iter()) {
                root_node.insert(x)
            }

            vec![root_node]
        } else {
            vec![]
        }
    }
}

impl Named for ComponentInterpreter {
    fn get_full_name(&self) -> &ir::Id {
        &self.full_name_clone
    }
}

impl Primitive for ComponentInterpreter {
    fn do_tick(&mut self) -> InterpreterResult<Vec<(ir::Id, Value)>> {
        let currently_done = self.done_is_high();

        // this component has been done for a cycle
        if currently_done {
            self.reset(&[])?;
        } else {
            self.step()?;
        }

        // just became done for an imperative component
        // so set done high
        if !currently_done && self.is_done() && self.interp.is_control() {
            self.set_done_high()
        }

        Ok(self.look_up_outputs())
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
    ) -> InterpreterResult<Vec<(ir::Id, crate::values::Value)>> {
        let mut assigned = HashSet::new();
        let mut input_vec = inputs
            .iter()
            .map(|(name, val)| {
                let port = self
                    .input_ports
                    .iter()
                    .find(|x| x.borrow().name == name)
                    .unwrap();
                assigned.insert(port.as_raw());
                (port.as_raw(), (*val).clone())
            })
            .collect::<Vec<_>>();

        for port in &self.input_ports {
            if !assigned.contains(&port.as_raw()) {
                let pt_ref = port.borrow();
                input_vec.push((
                    port.as_raw(),
                    Value::zeroes(pt_ref.width as usize),
                ));
            }
        }

        let mut env = self.get_env_mut();

        for (port, value) in input_vec {
            env.insert(port, value);
        }
        self.converge().unwrap();

        Ok(self.look_up_outputs())
    }

    fn reset(
        &mut self,
        _inputs: &[(ir::Id, &crate::values::Value)],
    ) -> InterpreterResult<Vec<(ir::Id, crate::values::Value)>> {
        if self.interp.is_control() {
            if !self.is_done() && !self.go_is_high() {
                return Ok(self.look_up_outputs());
            }
            assert!(
                self.is_done(),
                "Component {} interpreter reset before finishing",
                self.full_name_clone
            );
        }

        let interp = std::mem::take(&mut self.interp);

        let new = match interp {
            StructuralOrControl::Structural(mut s) => {
                s.step()?;
                StructuralOrControl::Structural(s)
            }
            StructuralOrControl::Control(control) => {
                let env = control.deconstruct()?;
                let control_interp = ControlInterpreter::new(
                    self.control_ref.clone(),
                    env,
                    &ComponentInfo::new(
                        self.comp_ref.continuous_assignments.clone(),
                        self.input_hash_set.clone(),
                        self.qual_name.clone(),
                    ),
                );
                control_interp.into()
            }
            _ => unreachable!(),
        };

        self.interp = new;

        if !self.interp._is_structural() {
            // only relevant for non-structural
            self.set_done_low();
        }

        Ok(self.look_up_outputs())
    }

    fn get_state(&self) -> Option<StateView<'_>> {
        Some(self.get_env())
    }

    fn serialize(
        &self,
        _signed: Option<PrintCode>,
    ) -> crate::serialization::Serializable {
        crate::serialization::Serializable::Full(
            self.get_env()
                .gen_serializer(matches!(_signed, Some(PrintCode::Binary))),
        )
    }

    fn get_comp_interpreter(&self) -> Option<&ComponentInterpreter> {
        Some(self)
    }
}
