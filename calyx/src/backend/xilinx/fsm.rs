use crate::utils;
use vast::v05::ast as v;

pub(crate) struct State<'a> {
    assigns: &'a [v::Expr],
    transition_condition: v::Expr,
}

/// A linear finite state machine
pub(crate) struct LinearFsm<'a> {
    prefix: String,
    states: Vec<State<'a>>,
}

impl<'a> LinearFsm<'a> {
    pub fn new<S>(prefix: S) -> Self
    where
        S: ToString,
    {
        Self {
            prefix: prefix.to_string(),
            states: Vec::new(),
        }
    }

    pub fn state<E>(
        mut self,
        assigns: &'a [v::Expr],
        transition_condition: E,
    ) -> Self
    where
        E: Into<v::Expr>,
    {
        self.add_state(assigns, transition_condition);
        self
    }

    pub fn add_state<E>(
        &mut self,
        assigns: &'a [v::Expr],
        transition_condition: E,
    ) where
        E: Into<v::Expr>,
    {
        self.states.push(State {
            assigns,
            transition_condition: transition_condition.into(),
        });
    }

    pub fn emit(self, module: &mut v::Module) {
        let num_states = self.states.len();
        let width = utils::math::bits_needed_for(num_states as u64);
        let state_reg = format!("{}state", self.prefix);
        let next_reg = format!("{}next", self.prefix);

        module.add_decl(v::Decl::new_reg(&state_reg, width));
        module.add_decl(v::Decl::new_reg(&next_reg, width));

        // fsm update block
        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::new_posedge("ACLK"));

        let mut ifelse = v::SequentialIfElse::new("ARESET".into());
        ifelse.add_seq(v::Sequential::new_nonblk_assign(
            state_reg.as_str().into(),
            v::Expr::new_int(0),
        ));
        ifelse.set_else(v::Sequential::new_nonblk_assign(
            state_reg.as_str().into(),
            next_reg.as_str().into(),
        ));

        parallel.add_seq(ifelse.into());
        module.add_stmt(parallel);

        let mut parallel = v::ParallelProcess::new_always();
        parallel.set_event(v::Sequential::Wildcard);

        let mut case = v::Case::new(state_reg.as_str().into());

        for (i, state) in self.states.into_iter().enumerate() {
            for assign in state.assigns {
                module.add_stmt(v::Parallel::Assign(
                    assign.clone(),
                    v::Expr::new_eq(
                        state_reg.as_str().into(),
                        v::Expr::new_int(i as i32),
                    ),
                ));
            }

            let this_state = i as i32;
            let next_state = ((i + 1) % num_states) as i32;

            let mut branch = v::CaseBranch::new(v::Expr::new_int(this_state));
            let mut ifelse =
                v::SequentialIfElse::new(state.transition_condition);
            ifelse.add_seq(v::Sequential::new_blk_assign(
                next_reg.as_str().into(),
                v::Expr::new_int(next_state),
            ));
            ifelse.set_else(v::Sequential::new_blk_assign(
                next_reg.as_str().into(),
                v::Expr::new_int(this_state),
            ));
            branch.add_seq(ifelse.into());
            case.add_branch(branch);
        }

        let mut default = v::CaseDefault::default();
        default.add_seq(v::Sequential::new_blk_assign(
            next_reg.as_str().into(),
            v::Expr::new_int(0),
        ));
        case.set_default(default);

        parallel.add_seq(v::Sequential::new_case(case));
        module.add_stmt(parallel);
    }
}
