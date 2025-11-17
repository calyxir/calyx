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
    pub inputs: Vec<PathRef>,
    pub outputs: Vec<PathRef>,
}

impl PlanResp {
    pub fn from_ir(
        mut ir: Ir,
        start_files: &[Utf8PathBuf],
        end_files: &[Utf8PathBuf],
    ) -> Self {
        PlanResp {
            inputs: start_files.iter().map(|f| ir.path_ref(f)).collect(),
            outputs: end_files.iter().map(|f| ir.path_ref(f)).collect(),
            ir,
        }
    }
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
pub fn ir_from_op_list(
    op_list: &Vec<(OpRef, Vec<StateRef>)>,
    req: &PlanReq,
    ops: &PrimaryMap<OpRef, Operation>,
    states: &PrimaryMap<StateRef, State>,
) -> Ir {
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
    let mut state_idx: SecondaryMap<StateRef, u32> = SecondaryMap::new();
    for &(op_ref, ref outputs) in op_list {
        let op = &ops[op_ref];
        let mut args = vec![];
        for &s in &op.input {
            let path: &Utf8PathBuf = if let Some(p) = input_files[s] {
                p
            } else {
                &format!("{}{}", states[s].name, state_idx[s]).into()
            };
            let r = ir.path_ref(path);
            args.push(r);
        }
        let mut rets = vec![];
        for &s in outputs {
            let path: &Utf8PathBuf = if let Some(p) = output_files[s] {
                p
            } else {
                state_idx[s] += 1;
                &format!("{}{}", states[s].name, state_idx[s]).into()
            };
            let r = ir.path_ref(path);
            rets.push(r);
        }

        ir.push(op_ref, &args, &rets);
    }
    ir
}
