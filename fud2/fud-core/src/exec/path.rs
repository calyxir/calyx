use cranelift_entity::{PrimaryMap, SecondaryMap};

use super::{OpRef, Operation, StateRef};

/// A `Step` is an op paired with its used outputs
type Step = (OpRef, Vec<StateRef>);

pub trait FindPath: std::fmt::Debug {
    /// Creates a sequence of `Step`s to take `start` to `end` using all operations in `through`
    ///
    /// `ops` is a complete list of operations.
    fn find_path(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>>;
}

#[derive(Debug, Default)]
pub struct EnumeratePathFinder {}
impl EnumeratePathFinder {
    const MAX_PATH_LEN: u32 = 6;

    pub fn new() -> Self {
        EnumeratePathFinder {}
    }

    fn try_paths_of_length<F>(
        plan: &mut Vec<(OpRef, Vec<StateRef>)>,
        len: u32,
        start: &[StateRef],
        end: &[StateRef],
        good: &F,
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>>
    where
        F: Fn(&[Step]) -> bool,
    {
        // check if the plan of given length is valid
        if len == 0 {
            return if good(plan) { Some(plan.clone()) } else { None };
        }

        // generate new plans over every loop
        for op_ref in ops.keys() {
            // make sure this op has its inputs created at some point
            // that op is also marked as used, later added ops prefered
            // TODO: consider just gening names here, might be easier
            let mut all_generated = true;
            for input in &ops[op_ref].input {
                let mut input_generated = false;
                for (o, outs) in plan.iter_mut().rev() {
                    if ops[*o].output.contains(input) {
                        input_generated = true;
                        if !outs.contains(input) {
                            outs.push(*input);
                        }
                        break;
                    }
                }
                all_generated &= input_generated || start.contains(input);
            }
            if !all_generated {
                continue;
            }

            // insert the op
            let outputs = ops[op_ref].output.clone().into_iter();
            let used_outputs =
                outputs.filter(|s| end.contains(s)).collect::<Vec<_>>();
            plan.push((op_ref, used_outputs));
            if let Some(plan) =
                Self::try_paths_of_length(plan, len - 1, start, end, good, ops)
            {
                return Some(plan);
            }
            plan.pop();
        }

        None
    }

    /// Creates a sequence of `Step`s to take `start` to `end` using all operations in `through`
    ///
    /// `ops` is a complete list of operations.
    fn find_path(
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        let good = |plan: &[(OpRef, Vec<StateRef>)]| {
            let end_created = end
                .iter()
                .all(|s| plan.iter().any(|(_, states)| states.contains(s)));

            // FIXME: Currently this checks that an outputs of an op specified by though is used.
            // However, it's possible that the only use of this output by another op whose outputs
            // are all unused. This means the plan doesn't actually use the specified op. but this
            // code reports it would.
            let through_used = through.iter().all(|t| {
                plan.iter()
                    .any(|(op, used_states)| op == t && !used_states.is_empty())
            });
            end_created && through_used
        };

        for len in 1..Self::MAX_PATH_LEN {
            if let Some(plan) = Self::try_paths_of_length(
                &mut vec![],
                len,
                start,
                end,
                &good,
                ops,
            ) {
                return Some(plan);
            }
        }
        None
    }
}

impl FindPath for EnumeratePathFinder {
    fn find_path(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        Self::find_path(start, end, through, ops)
    }
}

#[derive(PartialEq)]
enum Destination {
    State(StateRef),
    Op(OpRef),
}

#[derive(Debug, Default)]
pub struct SingleOpOutputPathFinder {}
impl SingleOpOutputPathFinder {
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
    pub fn find_path(
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

impl FindPath for SingleOpOutputPathFinder {
    fn find_path(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        assert!(start.len() == 1 && end.len() == 1);
        Self::find_path(start[0], end[0], through, ops)
    }
}
