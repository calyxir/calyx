use camino::Utf8PathBuf;
use cranelift_entity::{PrimaryMap, SecondaryMap};

use crate::{
    exec::{OpRef, Operation, State, StateRef},
    flang::Plan,
};

use super::planner::Request;

/// In the past, planners used to return a list of ops and the output states of that op which were
/// used. It turned out this wasn't enough information and planners also need to assign file paths
/// to these states, otherwise if an op which took in two files of the same state, it wouldn't know
/// which file to use for which arg of the op. This function converts from one of these lists of
/// ops to a `PlanResp`, assigning filenames to each state.
///
/// This conversion makes the assumption that there is only one input or output file for a given
/// state. This is required as otherwise, as described above, the function would have no way to
/// know which input or output file should be assigned to the given state.
pub fn prog_from_op_list(
    op_list: &Vec<(OpRef, Vec<StateRef>)>,
    req: &Request,
    ops: &PrimaryMap<OpRef, Operation>,
    states: &PrimaryMap<StateRef, State>,
) -> Plan {
    let input_files: SecondaryMap<StateRef, Option<&Utf8PathBuf>> = req
        .start_states
        .iter()
        .copied()
        .zip(req.start_files.iter().map(Some))
        .collect();
    let output_files: SecondaryMap<StateRef, Option<&Utf8PathBuf>> = req
        .end_states
        .iter()
        .copied()
        .zip(req.end_files.iter().map(Some))
        .collect();

    let mut plan = Plan::new();
    let mut state_idx: SecondaryMap<StateRef, u32> = SecondaryMap::new();
    for &(op_ref, ref op_outputs) in op_list {
        let op = &ops[op_ref];
        let mut args = vec![];
        for &s in &op.input {
            let r = if let Some(p) = input_files[s]
                && state_idx[s] == 0
            {
                let r = plan.path_ref(p);
                plan.push_input(r);
                r
            } else {
                let empty = "".to_string();
                let ext = states[s].extensions.first().unwrap_or(&empty);
                let r = plan.path_ref(
                    &Utf8PathBuf::from(format!(
                        "{}_{}",
                        states[s].name, state_idx[s]
                    ))
                    .with_extension(ext),
                );
                if req.start_states.contains(&s) {
                    plan.push_stdin(r);
                    plan.push_input(r);
                }
                r
            };
            args.push(r);
        }
        let mut rets = vec![];
        for &s in op_outputs {
            let r = if let Some(p) = output_files[s] {
                let r = plan.path_ref(p);
                plan.push_output(r);
                r
            } else {
                state_idx[s] += 1;
                let empty = "".to_string();
                let ext = states[s].extensions.first().unwrap_or(&empty);
                let r = plan.path_ref(
                    &Utf8PathBuf::from(format!(
                        "{}_{}",
                        states[s].name, state_idx[s]
                    ))
                    .with_extension(ext),
                );
                if req.end_states.contains(&s) {
                    plan.push_stdout(r);
                    plan.push_output(r);
                }
                r
            };
            rets.push(r);
        }

        plan.push(op_ref, &args, &rets);
    }
    plan
}
