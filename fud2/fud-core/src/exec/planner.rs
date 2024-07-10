use cranelift_entity::{PrimaryMap, SecondaryMap};

use super::{OpRef, Operation, StateRef};

/// `Step` is an op paired with its used outputs.
type Step = (OpRef, Vec<StateRef>);

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
    ) -> Option<Vec<Step>>;
}

#[derive(Debug, Default)]
pub struct EnumeratePlanner {}
impl EnumeratePlanner {
    pub fn new() -> Self {
        EnumeratePlanner {}
    }
    /// Returns a sequence of `Step`s to transform `start` to `end`. The `Step`s are guaranteed to
    /// contain all ops in `through`. If no such sequence exists, `None` is returned.
    ///
    /// `ops` is a complete list of operations.
    fn find_plan(
        _start: &[StateRef],
        _end: &[StateRef],
        _through: &[OpRef],
        _ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        todo!()
    }
}

impl FindPlan for EnumeratePlanner {
    fn find_plan(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        Self::find_plan(start, end, through, ops)
    }
}

#[derive(PartialEq)]
enum Destination {
    State(StateRef),
    Op(OpRef),
}

#[derive(Debug, Default)]
pub struct SingleOpOutputPlanner {}
impl SingleOpOutputPlanner {
    /// Find a chain of Operations from the `start` state to the `end`, which may be a state or the
    /// final operation in the chain.
    fn find_path_segment(
        start: StateRef,
        end: Destination,
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        // Our start state is the input.
        let mut visited = SecondaryMap::<StateRef, bool>::new();
        visited[start] = true;

        // Build the incoming edges for each vertex.
        let mut breadcrumbs = SecondaryMap::<StateRef, Option<OpRef>>::new();

        // Breadth-first search.
        let mut state_queue: Vec<StateRef> = vec![start];
        while !state_queue.is_empty() {
            let cur_state = state_queue.remove(0);

            // Finish when we reach the goal vertex.
            if end == Destination::State(cur_state) {
                break;
            }

            // Traverse any edge from the current state to an unvisited state.
            for (op_ref, op) in ops.iter() {
                if op.input[0] == cur_state && !visited[op.output[0]] {
                    state_queue.push(op.output[0]);
                    visited[op.output[0]] = true;
                    breadcrumbs[op.output[0]] = Some(op_ref);
                }

                // Finish when we reach the goal edge.
                if end == Destination::Op(op_ref) {
                    break;
                }
            }
        }

        // Traverse the breadcrumbs backward to build up the path back from output to input.
        let mut op_path: Vec<OpRef> = vec![];
        let mut cur_state = match end {
            Destination::State(state) => state,
            Destination::Op(op) => {
                op_path.push(op);
                ops[op].input[0]
            }
        };
        while cur_state != start {
            match breadcrumbs[cur_state] {
                Some(op) => {
                    op_path.push(op);
                    cur_state = ops[op].input[0];
                }
                None => return None,
            }
        }
        op_path.reverse();

        Some(
            op_path
                .iter()
                .map(|&op| (op, vec![ops[op].output[0]]))
                .collect::<Vec<_>>(),
        )
    }

    /// Find a chain of operations from the `start` state to the `end` state, passing through each
    /// `through` operation in order.
    pub fn find_plan(
        start: StateRef,
        end: StateRef,
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        let mut cur_state = start;
        let mut op_path = vec![];

        // Build path segments through each through required operation.
        for op in through {
            let segment =
                Self::find_path_segment(cur_state, Destination::Op(*op), ops)?;
            op_path.extend(segment);
            cur_state = ops[*op].output[0];
        }

        // Build the final path segment to the destination state.
        let segment =
            Self::find_path_segment(cur_state, Destination::State(end), ops)?;
        op_path.extend(segment);

        Some(op_path)
    }
}

impl FindPlan for SingleOpOutputPlanner {
    fn find_plan(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        assert!(start.len() == 1 && end.len() == 1);
        Self::find_plan(start[0], end[0], through, ops)
    }
}
