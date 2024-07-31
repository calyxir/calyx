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
    /// represents the absence of all states and all ops. A term `(root (states 3 x x) (ops x 2))`
    /// represents a pair of sets, the first containing a single state with `StateRef` 3 and the
    /// second containing a single op with `OpRef` 2.
    enum StateLanguage {
        // The root of a term. This is used to store a pair of "states" and "ops".
        "root" = Root([Id; 2]),
        // A list of states.
        "states" = States(Box<[Id]>),
        // A list of ops.
        "ops" = Ops(Box<[Id]>),
        // Symbolizes the absence of a state or op.
        "xxx" = X,
        // A ref, refering to either a StateRef or OpRef depending on context.
        Ref(u32),
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
                expr.add(StateLanguage::Ref(s.as_u32()))
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
                expr.add(StateLanguage::Ref(o.as_u32()))
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
/// `(root (states 1 2 3 xxx) (ops 1 xxx xxx)) => (root (states 1 xxx xxx r) (ops 1 2 xxx)`.
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
    // This enables retrieving the op later from the egraph.
    let name = op_ref.as_u32().to_string();

    let mut lhs: PatternAst<StateLanguage> = Default::default();
    // Collect states into a pattern.
    let state_ids = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if op.input.contains(s) {
                lhs.add(egg::ENodeOrVar::ENode(StateLanguage::Ref(s.as_u32())))
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
    // reduces the search space. This is why states go from a `Ref` to `X`.

    // Collect states into a pattern.
    let mut rhs: PatternAst<StateLanguage> = Default::default();
    let state_ids = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if op.output.contains(s) {
                rhs.add(egg::ENodeOrVar::ENode(StateLanguage::Ref(s.as_u32())))
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
                rhs.add(egg::ENodeOrVar::ENode(StateLanguage::Ref(o.as_u32())))
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
    ) -> Option<Vec<Step>> {
        // Collect all ops and states into sorted `Vec`s.
        let mut all_states: Vec<_> = ops
            .values()
            .map(|op| op.input.clone())
            .chain(ops.values().map(|op| op.output.clone()))
            .flatten()
            .collect();
        all_states.sort();
        all_states.dedup();

        let mut all_ops: Vec<_> = ops.keys().collect();
        all_ops.sort();
        all_ops.dedup();

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
        if runner.egraph.lookup_expr(&end_expr).is_some() {
            let mut explanation =
                runner.explain_equivalence(&start_expr, &end_expr);
            let explanation: Vec<_> = explanation
                .make_flat_explanation()
                .iter()
                .filter_map(|t| {
                    // Re-parse the op's reference from the rules creating `end_expr`.
                    t.forward_rule.map(|r| {
                        OpRef::from_u32(r.to_string().parse::<u32>().unwrap())
                    })
                })
                // Assume all outputs of an op are used. While this shouldn't cause any issues
                // of correctness, it ignores the problem of two ops outputting the same state.
                // This implies further reduction of the completeness of the search. This plan
                // leaves which state is prioritized undefined (though it should still be
                // deterministic).
                .map(|op_ref| (op_ref, ops[op_ref].output.to_vec()))
                .collect();
            if explanation.is_empty() {
                None
            } else {
                Some(explanation)
            }
        } else {
            None
        }
    }
}
