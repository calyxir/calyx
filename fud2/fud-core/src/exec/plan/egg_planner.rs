use super::{
    super::{OpRef, Operation, StateRef},
    FindPlan, Step,
};
use cranelift_entity::PrimaryMap;
use egg::{define_language, rewrite, Id, Pattern, RecExpr, Rewrite, Runner};

#[derive(Debug, Default)]
pub struct EggPlanner;

define_language! {
    /// A language to represent a collection of states.
    /// For example, if there are 3 states and 2 ops, a term `(root (states x x x) (ops x x))`
    /// represents the absense of all states and all ops. A term `(root (states 3 x x) (ops x 2))`
    /// represents a a pair of sets, the first containing a single state with `StateRef` 3 and the
    /// second containing a single op with `OpRef` 2.
    enum StateLanguage {
        // The root of a term. This is used to store a pair of "states" and "ops".
        "root" = Root([Id; 2]),
        // A list of states.
        "states" = States(Box<[Id]>),
        // A list of ops.
        "ops" = Ops(Box<[Id]>),
        // Symbolizes the absense of a state or op.
        "x" = X,
        // A ref, refering to either a StateRef or OpRef depending on context.
        Ref(u32),
    }
}

/// Construct string which can be parsed to a `root` term in `StateLanguage`.
///
/// `states` are in included states in the term, and `through` are the included ops. If
/// `states_use_x` is set, then all absent states will be marked with `x`. If it is not set, these
/// unused states will instead be marked with a unique variable to create a string parsable into a
/// pattern. `ops_use_x` is similar but applies to unused ops.
///
/// `all_states` is an ordered list of all states. `all_ops` is an ordered list of all ops.
fn language_string(
    states: &[StateRef],
    through: &[OpRef],
    states_use_x: bool,
    ops_use_x: bool,
    all_states: &[StateRef],
    all_ops: &[OpRef],
) -> String {
    // Collect states into a string.
    let state_str = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if states.contains(s) {
                s.as_u32().to_string()
            } else if states_use_x {
                String::from("x")
            } else {
                format!("?s{}", i)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let state_str = format!("(states {})", state_str);

    // Collect ops into a string.
    let op_str = all_ops
        .iter()
        .enumerate()
        .map(|(i, o)| {
            if through.contains(o) {
                o.as_u32().to_string()
            } else if ops_use_x {
                String::from("x")
            } else {
                format!("?o{}", i)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let op_str = format!("(ops {})", op_str);

    // Put them together with root.
    format!("(root {} {})", state_str, op_str)
}

/// Creates a `Rewrite` corresponding to `op` and `op_ref` for `StateLanguage`.
///
/// `through` is all of the ops required to be used by a plan the planner is attempting to find.
/// This is used as an optimization so only the ops specified in `through` are needed to be kept
/// track of in `ops` terms.
///
/// `all_states` is an ordered list of all states. `all_ops` is an ordered list of all ops.
fn rewrite_from_op(
    op_ref: OpRef,
    op: &Operation,
    through: &[OpRef],
    all_states: &[StateRef],
    all_ops: &[OpRef],
) -> Rewrite<StateLanguage, ()> {
    // Name the rewrite after the op's reference.
    // This is how we will retrieve it later from the egraph.
    let name = op_ref.as_u32().to_string();

    // Maintain lists of states in sorted order to reduce number of eclasses.
    let lhs: Pattern<StateLanguage> =
        language_string(&op.input, &[], false, false, all_states, all_ops)
            .parse()
            .unwrap();

    // The input states don't go away but this pretends they do because it massively
    // reduces the search space.

    // TODO: Change `language_string` so it can also be used to format `rhs`.
    // Collect states into a string.
    let state_str = all_states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if op.output.contains(s) {
                s.as_u32().to_string()
            } else if op.input.contains(s) {
                String::from("x")
            } else {
                format!("?s{}", i)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let state_str = format!("(states {})", state_str);

    // Collect ops into a string.
    let op_str = all_ops
        .iter()
        .enumerate()
        .map(|(i, o)| {
            if through.contains(o) {
                o.as_u32().to_string()
            } else {
                format!("?o{}", i)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let op_str = format!("(ops {})", op_str);

    let lang_str = format!("(root {} {})", state_str, op_str);
    let rhs: Pattern<StateLanguage> = lang_str.parse().unwrap();

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
        let all_states: Vec<_> = ops
            .values()
            .map(|op| op.input.clone())
            .chain(ops.values().map(|op| op.output.clone()))
            .flatten()
            .collect();
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
            language_string(start, &[], true, true, &all_states, &all_ops)
                .parse()
                .unwrap();

        // Find a solution.
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_expr(&start_expr)
            .run(&rules);

        // Create solution expression. This assumes that the ops generate exactly the requested
        // files with no extras.
        let end_expr: RecExpr<StateLanguage> =
            language_string(end, through, true, true, &all_states, &all_ops)
                .parse()
                .unwrap();

        // If the end expression exists, retrieve it using steps.
        if runner.egraph.lookup_expr(&end_expr).is_some() {
            let mut explanation = runner.explain_existance(&end_expr);
            Some(
                explanation
                    .make_flat_explanation()
                    .iter()
                    .filter_map(|t| {
                        // Re-parse the op's reference from the rules creating `end_expr`.
                        t.forward_rule.map(|r| {
                            OpRef::from_u32(
                                r.to_string().parse::<u32>().unwrap(),
                            )
                        })
                    })
                    // Assume all outputs of an op are used. While this shouldn't cause any issues
                    // of correctness, it ignores the problem of two ops outputing the same state.
                    // This implies further reduction of the completeness of the search. This plan
                    // leaves which state is prioritized undefined (though it should still be
                    // deterministic).
                    .map(|op_ref| (op_ref, ops[op_ref].output.to_vec()))
                    .collect(),
            )
        } else {
            None
        }
    }
}
