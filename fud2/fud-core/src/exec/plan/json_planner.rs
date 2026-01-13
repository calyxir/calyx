//! A planner which is predetermined by input from stdin. This always returns `None` for the plan
//! and must be special cased by the logic later. Currently planners only emit states with file
//! assignment done later.

use std::io::{self, Read as _};

use cranelift_entity::PrimaryMap;

use crate::{
    exec::{OpRef, Operation, State, StateRef},
    flang::{self, ast_to_prog},
};

use super::{FindPlan, planner::Request};

#[derive(Debug)]
pub struct JsonPlanner {}

impl FindPlan for JsonPlanner {
    fn find_plan(
        &self,
        _req: &Request,
        ops: &PrimaryMap<OpRef, Operation>,
        _states: &PrimaryMap<StateRef, State>,
    ) -> Option<flang::Prog> {
        let _ = _states;
        let mut stdin = io::stdin().lock();
        let mut input = String::new();
        let res = stdin.read_to_string(&mut input);
        if let Err(e) = res {
            eprintln!("error: {e}");
            return None;
        }

        let ast = &serde_json::from_str(&input);
        match ast {
            // Panicing here isn't great. The open issue to fix this is https://github.com/calyxir/calyx/issues/2610
            // In summery, it would be nice for planners to return `Result<PlanResp, SomeErrorType>`  so they could
            // better communicate how they fail.
            Err(e) => unimplemented!("{e}"),
            Ok(p) => Some(ast_to_prog(p, ops)),
        }
    }
}
