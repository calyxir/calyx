use camino::Utf8PathBuf;
use cranelift_entity::{PrimaryMap, SecondaryMap};

use crate::{
    exec::{self, State},
    flang::{Ir, PathRef},
};

use super::super::{OpRef, Operation, StateRef};

/// `Step` is an op paired with its used outputs.
pub type Step = (OpRef, Vec<StateRef>);

/// A reified function for finding a sequence of operations taking a start set of states to an end
/// set of states while guaranteing a set of "though" operations is used in the sequence.
pub trait FindPlan: std::fmt::Debug {
    /// Returns a sequence of `Step`s to transform `start` to `end`. The `Step`s are guaranteed to
    /// contain all ops in `through`. If no such sequence exists, `None` is returned.
    ///
    /// `ops` is a complete list of operations.
    fn find_plan(
        &self,
        req: &PlanReq,
        ops: &PrimaryMap<OpRef, Operation>,
        states: &PrimaryMap<StateRef, State>,
    ) -> Option<PlanResp>;
}

pub struct PlanReq<'a> {
    pub start_states: &'a [StateRef],
    pub end_states: &'a [StateRef],
    pub start_files: &'a [Utf8PathBuf],
    pub end_files: &'a [Utf8PathBuf],
    pub through: &'a [OpRef],
}

#[derive(Debug, PartialEq)]
pub struct PlanResp {
    pub ir: Ir,
    /// The input paths for `ir`.
    pub inputs: Vec<PathRef>,
    /// The output paths for `ir`.
    pub outputs: Vec<PathRef>,
    /// The paths in `inputs` which should be read from stdin.
    pub from_stdin: Vec<PathRef>,
    /// The paths in `outputs` which should be written to stdout.
    pub to_stdout: Vec<PathRef>,
}

impl<'a> From<&'a exec::Request> for PlanReq<'a> {
    fn from(value: &'a exec::Request) -> Self {
        PlanReq {
            start_states: &value.start_states,
            end_states: &value.end_states,
            start_files: &value.start_files,
            end_files: &value.end_files,
            through: &value.through,
        }
    }
}

/// This conversion makes the assumption that there is only one input or output file for a given
/// state. This is required as otherwise there is no way to know which input file pass to which op.
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
