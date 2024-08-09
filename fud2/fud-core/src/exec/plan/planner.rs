use cranelift_entity::PrimaryMap;

use crate::exec::State;

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
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
        states: &PrimaryMap<StateRef, State>,
    ) -> Option<Vec<Step>>;
}
