use camino::Utf8PathBuf;
use cranelift_entity::PrimaryMap;

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
        req: &Request,
        ops: &PrimaryMap<OpRef, Operation>,
        states: &PrimaryMap<StateRef, State>,
    ) -> Option<PlanResp>;
}

pub struct Request<'a> {
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

impl<'a> From<&'a exec::Request> for Request<'a> {
    fn from(value: &'a exec::Request) -> Self {
        Request {
            start_states: &value.start_states,
            end_states: &value.end_states,
            start_files: &value.start_files,
            end_files: &value.end_files,
            through: &value.through,
        }
    }
}
