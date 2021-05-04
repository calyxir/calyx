use crate::utils;
use std::collections::BTreeMap;
use vast::v05::ast as v;

pub(crate) struct State {
    assigns: Vec<v::Expr>,
    transition_condition: v::Expr,
}

/// A linear finite state machine
pub(crate) struct LinearFsm {
    state_reg: String,
    next_reg: String,
    states: Vec<State>,
    map: BTreeMap<String, usize>,
}

impl LinearFsm {
    pub fn new<S>(prefix: S) -> Self
    where
        S: AsRef<str>,
    {
        Self {
            state_reg: format!("{}state", prefix.as_ref()),
            next_reg: format!("{}next", prefix.as_ref()),
            states: Vec::new(),
            map: BTreeMap::new(),
        }
    }

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

    pub fn state_is(&self, state_name: &str) -> v::Expr {
        let idx = self.map[state_name];
        v::Expr::new_eq(
            self.state_reg.as_str().into(),
            v::Expr::new_int(idx as i32),
        )
    }

    pub fn emit(&self, module: &mut v::Module) {
        let num_states = self.states.len();
        let width = utils::math::bits_needed_for(num_states as u64);

        module.add_decl(v::Decl::new_reg(&self.state_reg, width));
        module.add_decl(v::Decl::new_reg(&self.next_reg, width));

        // fsm update block
        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::new_posedge("ACLK"));

        let mut ifelse = v::SequentialIfElse::new("ARESET".into());
        ifelse.add_seq(v::Sequential::new_nonblk_assign(
            self.state_reg.as_str().into(),
            v::Expr::new_int(0),
        ));
        ifelse.set_else(v::Sequential::new_nonblk_assign(
            self.state_reg.as_str().into(),
            self.next_reg.as_str().into(),
        ));

        parallel.add_seq(ifelse.into());
        module.add_stmt(parallel);

        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::Wildcard);

        let mut case = v::Case::new(self.state_reg.as_str().into());

        for (i, state) in self.states.iter().enumerate() {
            for assign in &state.assigns {
                module.add_stmt(v::Parallel::Assign(
                    assign.clone(),
                    v::Expr::new_eq(
                        self.state_reg.as_str().into(),
                        v::Expr::new_int(i as i32),
                    ),
                ));
            }

            let this_state = i as i32;
            let next_state = ((i + 1) % num_states) as i32;

            let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
            let mut ifelse =
                v::SequentialIfElse::new(state.transition_condition.clone());
            ifelse.add_seq(v::Sequential::new_blk_assign(
                self.next_reg.as_str().into(),
                v::Expr::new_int(next_state),
            ));
            ifelse.set_else(v::Sequential::new_blk_assign(
                self.next_reg.as_str().into(),
                v::Expr::new_int(this_state),
            ));
            branch.add_seq(ifelse.into());
            case.add_branch(branch);
        }

        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_blk_assign(
            self.next_reg.as_str().into(),
            v::Expr::new_int(0),
        ));
        case.set_default(default);

        parallel.add_seq(v::Sequential::new_case(case));
        module.add_stmt(parallel);
    }
}
