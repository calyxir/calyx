use calyx_utils as utils;
use std::collections::BTreeMap;
use vast::v05::ast as v;

pub(crate) struct State {
    assigns: Vec<v::Expr>,
    transition_condition: v::Expr,
}

/// A simple linear finite state machine
pub(crate) struct LinearFsm {
    state_reg: String,
    next_reg: String,
    clock: String,
    reset: v::Expr,
    states: Vec<State>,
    map: BTreeMap<String, usize>,
}

impl LinearFsm {
    /// Create a new fsm with the provided prefix, clock and reset.
    pub fn new<S, T, E>(prefix: S, clock: T, reset: E) -> Self
    where
        S: AsRef<str>,
        T: ToString,
        E: Into<v::Expr>,
    {
        Self {
            state_reg: format!("{}state", prefix.as_ref()),
            next_reg: format!("{}next", prefix.as_ref()),
            clock: clock.to_string(),
            reset: reset.into(),
            states: Vec::new(),
            map: BTreeMap::new(),
        }
    }

    /// Builder style method for adding a state to this FSM.
    pub fn state<E, S>(
        mut self,
        name: S,
        assigns: &[v::Expr],
        transition_condition: E,
    ) -> Self
    where
        E: Into<v::Expr>,
        S: ToString,
    {
        self.add_state(name, assigns, transition_condition);
        self
    }

    /// Add a state to this FSM.
    pub fn add_state<E, S>(
        &mut self,
        name: S,
        assigns: &[v::Expr],
        transition_condition: E,
    ) where
        E: Into<v::Expr>,
        S: ToString,
    {
        self.map.insert(name.to_string(), self.states.len());
        self.states.push(State {
            assigns: assigns.to_vec(),
            transition_condition: transition_condition.into(),
        });
    }

    /// Generate an expression representing the condition
    /// that the fsm is in the provided state.
    pub fn state_is(&self, state_name: &str) -> v::Expr {
        let idx = self.map[state_name];
        v::Expr::new_eq(self.state_reg.as_str(), idx as i32)
    }

    /// Generate an expression representing the condition
    /// that the fsm is in the provided state.
    pub fn next_state_is(&self, state_name: &str) -> v::Expr {
        let idx = self.map[state_name];
        v::Expr::new_eq(self.next_reg.as_str(), idx as i32)
    }

    /// Given a verilog module, emit the fsm.
    pub fn emit(&self, module: &mut v::Module) {
        let num_states = self.states.len();
        let width = utils::bits_needed_for(num_states as u64);

        module.add_decl(v::Decl::new_reg(&self.state_reg, width));
        module.add_decl(v::Decl::new_reg(&self.next_reg, width));

        // fsm update block
        module.add_stmt(super::utils::cond_non_blk_assign(
            &self.clock,
            self.state_reg.as_ref(),
            vec![
                (Some(self.reset.clone()), 0.into()),
                (None, self.next_reg.clone().into()),
            ],
        ));

        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::Wildcard);

        let mut case = v::Case::new(self.state_reg.as_str());

        for (i, state) in self.states.iter().enumerate() {
            for assign in &state.assigns {
                module.add_stmt(v::Parallel::Assign(
                    assign.clone(),
                    v::Expr::new_eq(self.state_reg.as_str(), i as i32),
                ));
            }

            let this_state = i as i32;
            let next_state = ((i + 1) % num_states) as i32;

            let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
            let mut ifelse =
                v::SequentialIfElse::new(state.transition_condition.clone());
            ifelse.add_seq(v::Sequential::new_blk_assign(
                self.next_reg.as_str(),
                next_state,
            ));
            ifelse.set_else(v::Sequential::new_blk_assign(
                self.next_reg.as_str(),
                this_state,
            ));
            branch.add_seq(ifelse);
            case.add_branch(branch);
        }

        let mut default = v::CaseDefault::default();
        default
            .add_seq(v::Sequential::new_blk_assign(self.next_reg.as_str(), 0));
        case.set_default(default);

        parallel.add_seq(v::Sequential::new_case(case));
        module.add_stmt(parallel);
    }
}
