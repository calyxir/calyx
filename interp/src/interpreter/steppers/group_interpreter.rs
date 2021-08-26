use super::super::interpret_group::{eval_prims, finish_interpretation};
use super::super::utils::{self, ConstCell, ConstPort};
use crate::environment::InterpreterState;
use crate::errors::{InterpreterError, InterpreterResult};
use crate::utils::{AsRaw, PortAssignment};
use crate::values::Value;
use calyx::ir::{self, Assignment, Cell, RRC};
use std::collections::HashSet;
use std::rc::Rc;

/// An internal wrapper enum which allows the Assignment Interpreter to own the
/// assignments it iterates over
enum AssignmentOwner<'a> {
    // first is always normal, second is always continuous
    Ref(Vec<&'a Assignment>, Vec<&'a Assignment>),
    Owned(Vec<Assignment>, Vec<Assignment>),
}

impl<'a> AssignmentOwner<'a> {
    // I'm sorry
    fn iter_all(&self) -> Box<dyn Iterator<Item = &Assignment> + '_> {
        match self {
            AssignmentOwner::Ref(assigns, cont) => {
                Box::new((*assigns).iter().chain((*cont).iter()).copied())
            }
            AssignmentOwner::Owned(assigns, cont) => {
                Box::new((*assigns).iter().chain((*cont).iter()))
            }
        }
    }

    // this is not currently used but may be relevant for mixed interpretation
    fn _iter_group_assigns(
        &self,
    ) -> Box<dyn Iterator<Item = &Assignment> + '_> {
        match self {
            AssignmentOwner::Ref(v1, _) => Box::new(v1.iter().copied()),
            AssignmentOwner::Owned(v1, _) => Box::new(v1.iter()),
        }
    }

    // this is not currently used but may be relevant for mixed interpretation
    fn _iter_cont(&self) -> Box<dyn Iterator<Item = &Assignment> + '_> {
        match self {
            AssignmentOwner::Ref(_, v2) => Box::new(v2.iter().copied()),
            AssignmentOwner::Owned(_, v2) => Box::new(v2.iter()),
        }
    }

    fn from_vecs((v1, v2): (Vec<Assignment>, Vec<Assignment>)) -> Self {
        Self::Owned(v1, v2)
    }
}

impl<'a, I1, I2> From<(I1, I2)> for AssignmentOwner<'a>
where
    I1: Iterator<Item = &'a Assignment>,
    I2: Iterator<Item = &'a Assignment>,
{
    fn from(iter: (I1, I2)) -> Self {
        Self::Ref(iter.0.collect(), iter.1.collect())
    }
}

/// An interpreter object which exposes a pausable interface to interpreting a
/// group of assignments
pub struct AssignmentInterpreter<'a> {
    state: InterpreterState,
    done_port: ConstPort,
    assigns: AssignmentOwner<'a>,
    cells: Vec<RRC<Cell>>,
    val_changed: Option<bool>,
}

impl<'a> AssignmentInterpreter<'a> {
    /// Creates a new AssignmentInterpreter which borrows the references to the
    /// assignments from an outside context
    pub fn new<I1, I2>(
        state: InterpreterState,
        done_signal: ConstPort,
        assigns: (I1, I2),
    ) -> Self
    where
        I1: Iterator<Item = &'a Assignment>,
        I2: Iterator<Item = &'a Assignment>,
    {
        let done_port = done_signal;
        let assigns: AssignmentOwner = assigns.into();
        let cells = utils::get_dest_cells(assigns.iter_all());

        Self {
            state,
            done_port,
            assigns,
            cells,
            val_changed: None,
        }
    }

    /// Creates a new AssignmentInterpreter which owns the assignments that it
    /// interpretes
    pub fn new_owned(
        state: InterpreterState,
        done_signal: ConstPort,
        vecs: (Vec<Assignment>, Vec<Assignment>),
    ) -> Self {
        let done_port = done_signal;
        let assigns: AssignmentOwner = AssignmentOwner::from_vecs(vecs);
        let cells = utils::get_dest_cells(assigns.iter_all());

        Self {
            state,
            done_port,
            assigns,
            cells,
            val_changed: None,
        }
    }

    /// Advance the stepper by a clock cycle
    pub fn step_cycle(&mut self) -> InterpreterResult<()> {
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

        let possible_ports: HashSet<*const ir::Port> =
            self.assigns.iter_all().map(|a| a.dst.as_raw()).collect();

        // this unwrap is safe
        while self.val_changed.unwrap() {
            let mut assigned_ports: HashSet<PortAssignment> = HashSet::new();
            self.val_changed = Some(false);

            let mut updates_list = vec![];
            // compute all updates from the assignments
            for assignment in self.assigns.iter_all() {
                if self.state.eval_guard(&assignment.guard) {
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
                let new_val = Value::from(0, old_val_width).unwrap();

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
        Ok(self.deconstruct())
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
        utils::is_signal_high(self.state.get_from_port(self.done_port))
    }

    pub fn deconstruct(self) -> InterpreterState {
        if self.is_deconstructable() {
            self.deconstruct_no_check()
        } else {
            panic!("Group simulation has not finished executing and cannot be deconstructed")
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
    pub fn reset<I: Iterator<Item = &'a ir::Assignment>>(
        self,
        assigns: I,
    ) -> InterpreterState {
        let done_signal = self.done_port;
        let env = self.deconstruct();

        finish_interpretation(env, done_signal, assigns).unwrap()
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
}
