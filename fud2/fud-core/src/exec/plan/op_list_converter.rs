use camino::Utf8PathBuf;
use cranelift_entity::{PrimaryMap, SecondaryMap};

use crate::{
    exec::{OpRef, Operation, State, StateRef},
    flang::Ir,
};

use super::{PlanReq, PlanResp};

/// In the past, planners used to return a list of ops and the output states of that op which were
/// used. It turned out this wasn't enough information and planners also need to assign file paths
/// to these states, otherwise if an op which took in two files of the same state, it wouldn't know
/// which file to use for which arg of the op. This function converts from one of these lists of
/// ops to a `PlanResp`, assigning filenames to each state.
///
/// This conversion makes the assumption that there is only one input or output file for a given
/// state. This is required as otherwise, as described above, the function would have no way to
/// know which input or output file should be assigned to the given state.
pub fn resp_from_op_list(
    op_list: &Vec<(OpRef, Vec<StateRef>)>,
    req: &PlanReq,
    ops: &PrimaryMap<OpRef, Operation>,
    states: &PrimaryMap<StateRef, State>,
) -> PlanResp {
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

    let mut ir = Ir::new();
    let mut inputs = vec![];
    let mut outputs = vec![];
    let mut from_stdin = vec![];
    let mut to_stdout = vec![];
    let mut state_idx: SecondaryMap<StateRef, u32> = SecondaryMap::new();
    for &(op_ref, ref op_outputs) in op_list {
        let op = &ops[op_ref];
        let mut args = vec![];
        for &s in &op.input {
            let r = if let Some(p) = input_files[s] {
                let r = ir.path_ref(p);
                inputs.push(r);
                r
            } else {
                let empty = "".to_string();
                let ext = states[s].extensions.first().unwrap_or(&empty);
                let r = ir.path_ref(
                    &Utf8PathBuf::from(format!(
                        "{}{}",
                        states[s].name, state_idx[s]
                    ))
                    .with_extension(ext),
                );
                if req.start_states.contains(&s) {
                    from_stdin.push(r);
                    inputs.push(r);
                }
                r
            };
            args.push(r);
        }
        let mut rets = vec![];
        for &s in op_outputs {
            let r = if let Some(p) = output_files[s] {
                let r = ir.path_ref(p);
                outputs.push(r);
                r
            } else {
                state_idx[s] += 1;
                let empty = "".to_string();
                let ext = states[s].extensions.first().unwrap_or(&empty);
                let r = ir.path_ref(
                    &Utf8PathBuf::from(format!(
                        "{}{}",
                        states[s].name, state_idx[s]
                    ))
                    .with_extension(ext),
                );
                if req.end_states.contains(&s) {
                    to_stdout.push(r);
                    outputs.push(r);
                }
                r
            };
            rets.push(r);
        }

        ir.push(op_ref, &args, &rets);
    }
    PlanResp {
        inputs,
        outputs,
        ir,
        to_stdout,
        from_stdin,
    }
}
