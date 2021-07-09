use super::super::simulation_utils::{self, ConstCell, ConstPort};
use super::super::working_environment::WorkingEnvironment;
use crate::environment::InterpreterState;
use crate::utils::get_const_from_rrc;
use crate::values::{OutputValue, ReadableValue, Value};
use calyx::ir::{self, Assignment, Cell, Port, RRC};
use itertools::Itertools;
use std::collections::HashSet;

pub struct AssignmentInterpreter<'a, 'b> {
    state: WorkingEnvironment,
    done_port: &'b Port,
    assigns: Vec<&'a Assignment>,
    cells: Vec<RRC<Cell>>,
    val_changed: Option<bool>,
}

impl<'a, 'b> AssignmentInterpreter<'a, 'b> {
    pub fn new<I>(
        env: InterpreterState,
        done_signal: &'b Port,
        assigns: I,
    ) -> Self
    where
        I: Iterator<Item = &'a Assignment>,
    {
        let state = env.into();
        let done_port = done_signal;
        let assigns = assigns.collect_vec();
        let cells = simulation_utils::get_dst_cells(assigns.iter().copied());

        Self {
            state,
            done_port,
            assigns,
            cells,
            val_changed: None,
        }
    }

    pub fn step_cycle(&mut self) {
        if !self.is_done()
            && self.val_changed.is_some()
            && !self.val_changed.unwrap()
        {
            self.state.do_tick();
            for cell in self.cells.iter() {
                if let Some(x) = self
                    .state
                    .backing_env
                    .cell_prim_map
                    .borrow_mut()
                    .get_mut(&(&cell.borrow() as &Cell as ConstCell))
                {
                    x.commit_updates()
                }
            }
            self.val_changed = None;
        }
    }

    pub fn step_convergence(&mut self) {
        // retain old value
        self.val_changed.get_or_insert(true);

        let possible_ports: HashSet<*const ir::Port> = self
            .assigns
            .iter()
            .map(|a| get_const_from_rrc(&a.dst))
            .collect();

        // this unwrap is safe
        while self.val_changed.unwrap() {
            let mut assigned_ports: HashSet<*const ir::Port> = HashSet::new();
            self.val_changed = Some(false);

            let mut updates_list = vec![];

            // compute all updates from the assignments
            for assignment in &self.assigns {
                // if assignment.dst.borrow().name == "done"
                // println!("{:?}", assignment.);
                if self.state.eval_guard(&assignment.guard) {
                    //if we change to smoosher, we need to add functionality that
                    //still prevents multiple drivers to same port, like below
                    //Perhaps use Smoosher's diff_other func?

                    //first check nothing has been assigned to this destination yet
                    if assigned_ports
                        .contains(&get_const_from_rrc(&assignment.dst))
                    {
                        let dst = assignment.dst.borrow();
                        panic!(
                        "[interpret_group]: multiple assignments to one port: {}.{}", dst.get_parent_name(), dst.name
                    );
                    }
                    //now add to the HS, because we are assigning
                    //regardless of whether value has changed this is still a
                    //value driving the port
                    assigned_ports.insert(get_const_from_rrc(&assignment.dst));
                    //ok now proceed
                    //the below (get) attempts to get from working_env HM first, then
                    //backing_env Smoosher. What does it mean for the value to be in HM?
                    //That it's a locked value?
                    let old_val = self.state.get(&assignment.dst.borrow());
                    let new_val_ref =
                        self.state.get_as_val(&assignment.src.borrow());

                    // no need to make updates if the value has not changed
                    let port = assignment.dst.clone(); // Rc clone
                    let new_val: OutputValue = new_val_ref.clone().into();

                    if old_val != new_val_ref.into() {
                        updates_list.push((port, new_val)); //no point in rewriting same value to this list

                        self.val_changed = Some(true);
                    }
                }
            }

            //now assign rest to 0
            //first get all that need to be 0
            for port in &possible_ports - &assigned_ports {
                //need to set to zero, because unassigned
                //ok now proceed

                //need to find appropriate-sized 0, so just read
                //width of old_val

                let old_val = self.state.get_as_val_const(port);
                let old_val_width = old_val.width(); //&assignment.dst.borrow().width()
                let new_val: OutputValue =
                    Value::try_from_init(0, old_val_width).unwrap().into();
                //updates_list.push((port, new_val));

                //how to avoid infinite loop?
                //if old_val is imm value and zero, then that's
                //when val_changed_flag is false, else true.
                if old_val.as_u64() != 0 {
                    self.val_changed = Some(true);
                }

                //update directly
                self.state.update_val_const_port(port, new_val);
            }

            // perform all the updates
            for (port, value) in updates_list {
                self.state.update_val(&port.borrow(), value);
            }

            let changed = self.state.eval_prims(self.cells.iter(), false);
            if changed {
                self.val_changed = Some(true);
            }
        }
    }
    pub fn step_cycle_convergence(&mut self) {
        self.step_cycle();
        self.step_convergence();
    }

    pub fn run_group(&mut self) {
        while !self.is_done() {
            self.step_cycle_convergence();
        }
        self.step_convergence();
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        simulation_utils::is_signal_high(self.state.get(self.done_port))
    }

    pub fn deconstruct_no_check(self) -> InterpreterState {
        if self.is_deconstructable() {
            self.state.collapse_env(false)
        } else {
            panic!("Group simulation has not finished executing and cannot be deconstructed")
        }
    }

    fn is_deconstructable(&self) -> bool {
        self.is_done()
            && self.val_changed.is_some()
            && !self.val_changed.unwrap()
    }

    /// Concludes interpretation to a group, effectively setting the go signal low
    /// for a given group. This function updates the values in the environment
    /// accordingly using zero as a placeholder for values that are undefined
    pub fn finish_interpretation<I: Iterator<Item = &'a ir::Assignment>>(
        mut env: InterpreterState,
        done_signal: &ir::Port,
        assigns: I,
    ) -> InterpreterState {
        // replace port values for all the assignments
        let assigns = assigns.collect::<Vec<_>>();

        for &ir::Assignment { dst, .. } in &assigns {
            env.insert(
                &dst.borrow() as &ir::Port as ConstPort,
                Value::zeroes(dst.borrow().width as usize),
            );
        }

        let cells = simulation_utils::get_dst_cells(assigns.iter().copied());

        env.insert(done_signal as ConstPort, Value::bit_low());
        let mut working_env: WorkingEnvironment = env.into();
        working_env.eval_prims(cells.iter(), true);

        working_env.collapse_env(false)
    }
}
