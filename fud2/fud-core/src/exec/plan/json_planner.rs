//! A planner which is predetermined by input from stdin. This always returns `None` for the plan
//! and must be special cased by the logic later. Currently planners only emit states with file
//! assignment done later.

use std::io::{self, Read as _};

use cranelift_entity::PrimaryMap;

use crate::{
    exec::{OpRef, Operation, State, StateRef},
    flang::ast_to_ir,
};

use super::{FindPlan, PlanReq, planner::PlanResp};

#[derive(Debug)]
pub struct JsonPlanner {}

impl FindPlan for JsonPlanner {
    fn find_plan(
        &self,
        req: &PlanReq,
        ops: &PrimaryMap<OpRef, Operation>,
        _states: &PrimaryMap<StateRef, State>,
    ) -> Option<PlanResp> {
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
            Err(e) => unimplemented!("{e}"),
            Ok(ast) => {
                let mut ir = ast_to_ir(ast, ops);
                Some(PlanResp {
                    inputs: req
                        .start_files
                        .iter()
                        .map(|f| ir.path_ref(f))
                        .collect(),
                    outputs: req
                        .end_files
                        .iter()
                        .map(|f| ir.path_ref(f))
                        .collect(),
                    ir,
                    to_stdout: vec![],
                    from_stdin: vec![],
                })
            }
        }
    }
}
