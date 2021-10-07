use super::super::interpret_group::{eval_prims, finish_interpretation};
use super::super::utils::{self, ConstCell, ConstPort};
use crate::environment::InterpreterState;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::utils::{AsRaw, PortAssignment};
use crate::values::Value;
use calyx::ir::{self, Assignment, Cell, RRC};
use std::cell::Ref;
use std::collections::HashSet;
use std::rc::Rc;

use super::control_interpreter::EnableHolder;
use crate::interpreter_ir as iir;

pub enum AssignmentHolder {
    CombGroup(RRC<ir::CombGroup>),
    Group(RRC<ir::Group>),
    Vec(Rc<Vec<Assignment>>),
}

impl Default for AssignmentHolder {
    fn default() -> Self {
        Self::Vec(Rc::new(Vec::new()))
    }
}

impl From<RRC<ir::CombGroup>> for AssignmentHolder {
    fn from(input: RRC<ir::CombGroup>) -> Self {
        Self::CombGroup(input)
    }
}

impl From<RRC<ir::Group>> for AssignmentHolder {
    fn from(gr: RRC<ir::Group>) -> Self {
        Self::Group(gr)
    }
}

impl From<Vec<Assignment>> for AssignmentHolder {
    fn from(v: Vec<Assignment>) -> Self {
        Self::Vec(Rc::new(v))
    }
}

impl From<Rc<Vec<Assignment>>> for AssignmentHolder {
    fn from(v: Rc<Vec<Assignment>>) -> Self {
        Self::Vec(v)
    }
}

impl From<EnableHolder> for AssignmentHolder {
    fn from(en: EnableHolder) -> Self {
        match en {
            EnableHolder::Group(grp) => AssignmentHolder::Group(grp),
            EnableHolder::CombGroup(cgrp) => AssignmentHolder::CombGroup(cgrp),
            EnableHolder::Vec(v) => AssignmentHolder::Vec(v),
        }
    }
}

impl AssignmentHolder {
    pub fn get_ref(&self) -> IterRef<'_> {
        match self {
            AssignmentHolder::CombGroup(cg) => IterRef::CombGroup(cg.borrow()),
            AssignmentHolder::Group(grp) => IterRef::Group(grp.borrow()),
            AssignmentHolder::Vec(v) => IterRef::Vec(v),
        }
    }
    pub fn get_name(&self) -> Option<ir::Id> {
        match self {
            AssignmentHolder::CombGroup(cg) => Some(cg.borrow().name().clone()),
            AssignmentHolder::Group(g) => Some(g.borrow().name().clone()),
            AssignmentHolder::Vec(_) => None,
        }
    }
}

pub enum IterRef<'a> {
    CombGroup(Ref<'a, ir::CombGroup>),
    Group(Ref<'a, ir::Group>),
    Vec(&'a Rc<Vec<Assignment>>),
}

impl<'a> IterRef<'a> {
    pub fn iter(&self) -> Box<dyn Iterator<Item = &ir::Assignment> + '_> {
        match self {
            IterRef::CombGroup(cg) => Box::new(cg.assignments.iter()),
            IterRef::Group(g) => Box::new(g.assignments.iter()),
            IterRef::Vec(v) => Box::new(v.iter()),
        }
    }
}

/// An interpreter object which exposes a pausable interface to interpreting a
/// group of assignments
pub struct AssignmentInterpreter {
    state: InterpreterState,
    done_port: Option<ConstPort>,
    assigns: AssignmentHolder,
    cont_assigns: iir::ContinuousAssignments,
    cells: Vec<RRC<Cell>>,
    val_changed: Option<bool>,
}

impl AssignmentInterpreter {
    /// Creates a new AssignmentInterpreter which borrows the references to the
    /// assignments from an outside context
    pub fn new<A: Into<AssignmentHolder>>(
        state: InterpreterState,
        done_signal: Option<RRC<ir::Port>>,
        assigns: A,
        cont_assigns: &Rc<Vec<ir::Assignment>>,
    ) -> Self {
        let done_port = done_signal.as_ref().map(|x| x.as_raw());
        let assigns: AssignmentHolder = assigns.into();
        let cells = utils::get_dest_cells(
            assigns.get_ref().iter().chain(cont_assigns.iter()),
            done_signal,
        );

        Self {
            state,
            done_port,
            assigns,
            cont_assigns: Rc::clone(cont_assigns),
            cells,
            val_changed: None,
        }
    }

    /// Advance the stepper by a clock cycle
    pub fn step_cycle(&mut self) -> InterpreterResult<()> {
        //TODO (Griffin): Make sure this does the convergence step first if needed, rather
        // than just skipping
        if !self.is_done()
            && self.val_changed.is_some()
            && !self.val_changed.unwrap()
        {
            let mut update_list: Vec<(RRC<ir::Port>, Value)> = vec![];

            for cell in self.cells.iter() {
                if let Some(x) = self
                    .state
                    .cell_map
                    .borrow_mut()
                    .get_mut(&(&cell.borrow() as &Cell as ConstCell))
                {
                    let new_vals = x.do_tick();
                    for (port, val) in new_vals {
                        let port_ref = cell.borrow().find(port).unwrap();

                        update_list.push((Rc::clone(&port_ref), val));
                    }
                }
            }

            for (port, val) in update_list {
                self.state.insert(port, val);
            }
            self.val_changed = None;
        }
        Ok(())
    }

    /// Continue interpreting the assignments until the combinational portions
    /// converge
    pub fn step_convergence(&mut self) -> InterpreterResult<()> {
        // retain old value
        self.val_changed.get_or_insert(true);

        let possible_ports: HashSet<*const ir::Port> = self
            .assigns
            .get_ref()
            .iter()
            .chain(self.cont_assigns.iter())
            .map(|a| a.dst.as_raw())
            .collect();

        // this unwrap is safe
        while self.val_changed.unwrap() {
            let mut assigned_ports: HashSet<PortAssignment> = HashSet::new();
            self.val_changed = Some(false);

            let mut updates_list = vec![];

            let assign_ref = self.assigns.get_ref();
            // compute all updates from the assignments
            for assignment in assign_ref.iter().chain(self.cont_assigns.iter())
            {
                if self.state.eval_guard(&assignment.guard)? {
                    let pa = PortAssignment::new(assignment);
                    //first check nothing has been assigned to this destination yet
                    if let Some(prior_assign) = assigned_ports.get(&pa) {
                        let s_orig = prior_assign.get_assignment();
                        let s_conf = pa.get_assignment();

                        let dst = assignment.dst.borrow();

                        return Err(InterpreterError::conflicting_assignments(
                            dst.name.clone(),
                            dst.get_parent_name(),
                            s_orig,
                            s_conf,
                        ));
                    }
                    //now add to the HS, because we are assigning
                    //regardless of whether value has changed this is still a
                    //value driving the port
                    assigned_ports.insert(pa);
                    //ok now proceed
                    //the below (get) attempts to get from working_env HM first, then
                    //backing_env Smoosher. What does it mean for the value to be in HM?
                    //That it's a locked value?
                    let old_val =
                        self.state.get_from_port(&assignment.dst.borrow());
                    let new_val_ref =
                        self.state.get_from_port(&assignment.src.borrow());
                    // no need to make updates if the value has not changed
                    let port = assignment.dst.clone(); // Rc clone
                    let new_val = new_val_ref.clone();

                    if old_val != new_val_ref {
                        updates_list.push((port, new_val)); //no point in rewriting same value to this list
                        self.val_changed = Some(true);
                    }
                }
            }

            let assigned_const_ports: HashSet<_> = assigned_ports
                .iter()
                .map(PortAssignment::get_port)
                .collect();

            //now assign rest to 0
            //first get all that need to be 0
            for port in &possible_ports - &assigned_const_ports {
                //need to set to zero, because unassigned
                //ok now proceed

                //need to find appropriate-sized 0, so just read
                //width of old_val

                let old_val = self.state.get_from_port(port);
                let old_val_width = old_val.width(); //&assignment.dst.borrow().width()
                let new_val = Value::from(0, old_val_width);

                if old_val.as_u64() != 0 {
                    self.val_changed = Some(true);
                }

                //update directly
                self.state.insert(port, new_val);
            }

            // perform all the updates
            for (port, value) in updates_list {
                self.state.insert(port, value);
            }

            let changed = eval_prims(&mut self.state, self.cells.iter(), false);
            if changed {
                self.val_changed = Some(true);
            }
        }
        Ok(())
    }
    /// Advance the interpreter by a cycle, if possible
    pub fn step(&mut self) -> InterpreterResult<()> {
        self.step_cycle()?;
        self.step_convergence()
    }

    /// Run interpreter until it is finished executing and return the output
    /// environment
    pub fn run_and_deconstruct(
        mut self,
    ) -> InterpreterResult<InterpreterState> {
        self.run()?;
        self.deconstruct()
    }

    /// Run the interpreter until it finishes executing
    pub fn run(&mut self) -> InterpreterResult<()> {
        while !self.is_done() {
            self.step()?;
        }
        self.step_convergence()
    }

    #[inline]
    fn is_done(&self) -> bool {
        self.done_port.is_none()
            || utils::is_signal_high(
                self.state.get_from_port(self.done_port.unwrap()),
            )
    }

    pub fn deconstruct(self) -> InterpreterResult<InterpreterState> {
        if self.is_deconstructable() {
            Ok(self.deconstruct_no_check())
        } else if let Some(name) = self.assigns.get_name() {
            Err(InterpreterError::InvalidGroupExitNamed(name))
        } else {
            Err(InterpreterError::InvalidGroupExitUnnamed)
        }
    }

    #[inline]
    fn deconstruct_no_check(self) -> InterpreterState {
        self.state
    }

    pub fn is_deconstructable(&self) -> bool {
        self.is_done()
            && self.val_changed.is_some()
            && !self.val_changed.unwrap()
    }

    /// The inerpreter must have finished executing first
    pub fn reset(mut self) -> InterpreterResult<InterpreterState> {
        let assigns = std::mem::take(&mut self.assigns);
        let done_signal = self.done_port;
        let env = self.deconstruct()?;

        let assign_ref = assigns.get_ref();

        // note there might be some trouble with mixed assignments
        finish_interpretation(env, done_signal, assign_ref.iter())
    }

    pub fn get<P: AsRaw<ir::Port>>(&self, port: P) -> &Value {
        self.state.get_from_port(port)
    }

    // This is not currenty relevant for anything, but may be needed later
    // pending adjustments to the primitive contract as we will need the ability
    // to pass new inputs to components
    pub(super) fn _insert<P: AsRaw<ir::Port>>(&mut self, port: P, val: Value) {
        self.state.insert(port, val)
    }

    pub fn get_env(&self) -> &InterpreterState {
        &self.state
    }

    pub fn get_mut_env(&mut self) -> &mut InterpreterState {
        &mut self.state
    }
}
