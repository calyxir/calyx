//! A planner based around egraphs.
//!
//! ## Problem Statement for Planners
//! The planner is given a set of "ops" and a set of "states." Ops are function taking in a set of
//! states and returning a set of states.
//!
//! The planner is tasked with finding a sequence of ops which takes some inital set of states to a
//! new set of states with the constraint that there is a set of ops which must be used in this
//! sequence. By being "used" in this sequence, this means each op must be present in the sequence
//! and it's outputs, if they were removed, would make construction of the final set of output
//! states impossible.
//!
//! If states are though of ast types, then this problem reduces to creating a function out of an
//! API, the ops, which satisfies a given type signature.
//!
//! ## Encoding in Terms
//! This module is an optimization over a simple enumerative solution for finding these composition
//! of functions. The space of possible programs are first encoded as a pair of sets.
//!
//! Assign each op and state a unique index. Then, a set of states which was created by a sequence
//! of ops can be represented by a list of "x"s and "c"s (two bit vectors if that feels more
//! intuitive). The ith element of the first list is a "c" iff state i exists in the se tof states,
//! else it is "x". Similarly, the ith element of the second list is "c" iff op i was used to
//! create a set of states, else it is "x".
//!
//! For example, if there are 3 states and 2 ops, the sets of states {1, 3} created using op 2,
//! could be represented by the string (c x c) (x c).
//!
//! Ops can then be described as rewrites to this string. For example, if op 1 takes state 1 to
//! state 2, it would do the following to the above string: (c x c) (x c) => (x c c) (c c). An
//! important observation is the op "consumed" state 1. This is no a requirement for correctness.
//! Ops do not consume their input states. However, this is done when creating rewrites because
//! otherwise the size of the e-graph becomes infeasible.
//!
//! ## The Algorithm
//! The input states are encoded as a pair of lists. They are created from no ops so the second
//! list contains only "x".
//!
//! Ops are then encoded as rewrites and applied repeatedly to the e-graph.
//! Equality saturation is a natural place to stop, take below, but not necessary (and possibly
//! prohibitive for large op graphs).
//!
//! The desired outputs are then encoded as a pair of lists, the ops in the second list those
//! required to be in the output sequence. If this pair of lists is found in the final e-graph, a
//! solution exists and can be retrieved from the rewrites (egg, what this module uses to represent
//! e-graphs supports getting a sequence of rewrites proving a term in an e-graph exists).
//! Otherwise, "no solution found" is returned.
//!
//! ## Comments
//! This solution isn't complete, but it is correct and works efficiently for the current and
//! somewhat larger op graph sizes. This cases it excludes are where a single state is used by
//! multiple ops.

use crate::exec::State;

use super::{
    super::{OpRef, Operation, StateRef},
    FindPlan, Step,
};
use cranelift_entity::PrimaryMap;
use egg::{
    define_language, rewrite, Id, Pattern, PatternAst, RecExpr, Rewrite, Runner,
};

#[derive(Debug, Default)]
pub struct EggPlanner;

define_language! {
    /// A language to represent a collection of states.
    /// For example, if there are 3 states and 2 ops, a term `(root (states x x x) (ops x x))`
    /// represents the absence of all states and all ops. A term `(root (states c x x) (ops x c))`
    /// represents a pair of sets, the first containing a single state with index 3 and the second
    /// containing a single op with index 2.
    ///
    /// Here indexes are an arbitrary (but consistent!) mapping from states and ops to non-negative
    /// integers.
    enum StateLanguage {
        // The root of a term. This is used to store a pair of "states" and "ops".
        "root" = Root([Id; 2]),
        // A list of states.
        "states" = States(Box<[Id]>),
        // A list of ops.
        "ops" = Ops(Box<[Id]>),
        // Symbolizes the absence of a state or op.
        "xxx" = X,
        // Symbolizes the presence of a state or op.
        "ccc" = C,
    }
}

/// Construct an expr for `root` term in `StateLanguage`.
///
/// `states` are in included states in the term, and `through` are the included ops. Unspecified
/// ops and states are maked with "xxx".
/// `all_states` is an ordered list of all states. `all_ops` is an ordered list of all ops.
fn language_expr(
    states: &[StateRef],
    through: &[OpRef],
    all_states: &[StateRef],
    all_ops: &[OpRef],
) -> RecExpr<StateLanguage> {
    let mut expr: RecExpr<StateLanguage> = Default::default();
    // Collect states into an expr.
    let state_ids = all_states
        .iter()
        .map(|s| {
            if states.contains(s) {
                expr.add(StateLanguage::C)
            } else {
                expr.add(StateLanguage::X)
            }
        })
        .collect::<Vec<_>>();
    let state_expr =
        expr.add(StateLanguage::States(state_ids.into_boxed_slice()));

    // Collect ops into an expr.
    let op_ids = all_ops
        .iter()
        .map(|o| {
            if through.contains(o) {
                expr.add(StateLanguage::C)
            } else {
                expr.add(StateLanguage::X)
            }
        })
        .collect::<Vec<_>>();
    let op_expr = expr.add(StateLanguage::Ops(op_ids.into_boxed_slice()));

    expr.add(StateLanguage::Root([state_expr, op_expr]));
    expr
}

/// Creates a `Rewrite` corresponding to `op` and `op_ref` for `StateLanguage`.
///
/// `through` is all of the ops required to be used by a plan the planner is attempting to find.
/// This is used as an optimization so only the ops specified in `through` are needed to be kept
/// track of in `ops` terms.
///
/// `all_states` is an ordered list of all states. `all_ops` is an ordered list of all ops.
///
/// This rewrite will look like `(root (states <state list>) (ops <op list>)) =>
/// (root (states <state list - op inputs + op outputs>) (ops <op list + op>))`
///
/// As an example, given 3 ops and 4 states, if op 2 takes states 2 and 3 to state 4, the rewrite
/// would take
/// `(root (states c c c xxx) (ops c xxx xxx)) => (root (states c xxx xxx c) (ops c c xxx))`.
///
/// This function removes input states from the set of those available not because they are
/// consumed, but because it massively reduces the search space. This is a concession of
/// completeness for efficiency.
fn rewrite_from_op(
    op_ref: OpRef,
    op: &Operation,
    through: &[OpRef],
    all_states: &[StateRef],
    all_ops: &[OpRef],
) -> Rewrite<StateLanguage, ()> {
    // Name the rewrite after the op's reference.
    // This enables retrieving the op later from the e-graph.
    let name = op_ref.as_u32().to_string();

    let mut lhs: PatternAst<StateLanguage> = Default::default();
    // Collect states into a pattern.
    let state_ids = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if op.input.contains(s) {
                lhs.add(egg::ENodeOrVar::ENode(StateLanguage::C))
            } else {
                lhs.add(egg::ENodeOrVar::Var(
                    format!("?s{}", i).parse().unwrap(),
                ))
            }
        })
        .collect::<Vec<_>>();
    let state_pattern = lhs.add(egg::ENodeOrVar::ENode(StateLanguage::States(
        state_ids.into_boxed_slice(),
    )));

    // Collect ops into a pattern.
    let op_ids = all_ops
        .iter()
        .enumerate()
        .map(|(i, _o)| {
            lhs.add(egg::ENodeOrVar::Var(format!("?o{}", i).parse().unwrap()))
        })
        .collect::<Vec<_>>();
    let op_pattern = lhs.add(egg::ENodeOrVar::ENode(StateLanguage::Ops(
        op_ids.into_boxed_slice(),
    )));

    lhs.add(egg::ENodeOrVar::ENode(StateLanguage::Root([
        state_pattern,
        op_pattern,
    ])));
    let lhs = Pattern::new(lhs);

    // The input states don't go away but this pretends they do because it massively
    // reduces the search space. This is why states go from a `C` to `X`.

    // Collect states into a pattern.
    let mut rhs: PatternAst<StateLanguage> = Default::default();
    let state_ids = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if op.output.contains(s) {
                rhs.add(egg::ENodeOrVar::ENode(StateLanguage::C))
            } else if op.input.contains(s) {
                rhs.add(egg::ENodeOrVar::ENode(StateLanguage::X))
            } else {
                rhs.add(egg::ENodeOrVar::Var(
                    format!("?s{}", i).parse().unwrap(),
                ))
            }
        })
        .collect::<Vec<_>>();
    let state_pattern = rhs.add(egg::ENodeOrVar::ENode(StateLanguage::States(
        state_ids.into_boxed_slice(),
    )));

    // Collect ops into a pattern.
    let op_ids = all_ops
        .iter()
        .enumerate()
        .map(|(i, &o)| {
            if through.contains(&op_ref) && o == op_ref {
                rhs.add(egg::ENodeOrVar::ENode(StateLanguage::C))
            } else {
                rhs.add(egg::ENodeOrVar::Var(
                    format!("?o{}", i).parse().unwrap(),
                ))
            }
        })
        .collect::<Vec<_>>();
    let op_pattern = rhs.add(egg::ENodeOrVar::ENode(StateLanguage::Ops(
        op_ids.into_boxed_slice(),
    )));

    rhs.add(egg::ENodeOrVar::ENode(StateLanguage::Root([
        state_pattern,
        op_pattern,
    ])));
    let rhs = Pattern::new(rhs);

    rewrite!(name; lhs => rhs)
}

impl FindPlan for EggPlanner {
    fn find_plan(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
        states: &PrimaryMap<StateRef, State>,
    ) -> Option<Vec<Step>> {
        // Collect all ops and states into sorted `Vec`s.
        let all_states: Vec<_> = states.keys().collect();
        let all_ops: Vec<_> = ops.keys().collect();

        // Construct egg rewrites for each op.
        let rules: Vec<Rewrite<StateLanguage, ()>> = ops
            .iter()
            .map(|(op_ref, op)| {
                rewrite_from_op(op_ref, op, through, &all_states, &all_ops)
            })
            .collect();

        // Construct initial expression.
        let start_expr: RecExpr<StateLanguage> =
            language_expr(start, &[], &all_states, &all_ops);

        // Find a solution.
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_expr(&start_expr)
            .run(&rules);

        // Create solution expression. This assumes that the ops generate exactly the requested
        // files with no extras.
        let end_expr: RecExpr<StateLanguage> =
            language_expr(end, through, &all_states, &all_ops);

        // If the end expression exists, retrieve it using steps.
        runner
            .egraph
            .lookup_expr(&end_expr)
            .map(|_| {
                runner
                    .explain_equivalence(&start_expr, &end_expr)
                    .make_flat_explanation()
                    .iter()
                    .filter_map(|t| t.forward_rule)
                    .map(|r|
                        // Re-parse the op's reference from the rules creating `end_expr`.
                        OpRef::from_u32(r.to_string().parse::<u32>().unwrap()))
                    // Assume all outputs of an op are used. While this shouldn't cause any issues
                    // of correctness, it ignores the problem of two ops outputting the same state.
                    // This implies further reduction of the completeness of the search. This plan
                    // leaves which state is prioritized undefined (though it should still be
                    // deterministic).
                    .map(|op_ref| (op_ref, ops[op_ref].output.to_vec()))
                    .collect::<Vec<_>>()
            })
            .filter(|steps| !steps.is_empty())
    }
}
