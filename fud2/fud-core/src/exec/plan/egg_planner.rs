use std::collections::BTreeSet;

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
    enum StateLanguage {
        // The root of a term
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

/// Construct string which can be parsed to a `StateLanguage` expression out of a set of
/// `StateRef`. If `use_x` is set, states not in `states` and ops not in `through` will be replaced
/// with "x", else they will be given a unique variable name.
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
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let all_ops: Vec<_> =
            ops.keys().collect::<BTreeSet<_>>().into_iter().collect();

        // Construct egg rewrites for each op.
        let rules: Vec<Rewrite<StateLanguage, ()>> = ops
            .iter()
            .map(|(op_ref, op)| {
                // Name the rewrite after the op's reference.
                // This is how we will retrieve it later from the egraph.
                let name = op_ref.as_u32().to_string();

                // Maintain lists of states in sorted order to reduce number of eclasses.
                let lhs: Pattern<StateLanguage> = language_string(
                    &op.input,
                    &[],
                    false,
                    false,
                    &all_states,
                    &all_ops,
                )
                .parse()
                .unwrap();

                // The input states don't go away but this pretends they do because it massively
                // reduces the search space.

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

        // Check if a solution exists.
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
                    // Assume all outputs of an op are used.
                    // TODO: Make it so only a subset of the outputs of a thing need to be used.
                    .map(|op_ref| (op_ref, ops[op_ref].output.to_vec()))
                    .collect(),
            )
        } else {
            None
        }
    }
}
