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
        // A list of states.
        "states" = List(Box<[Id]>),
        // A singular state, represented by its StateRef.
        State(u32),
    }
}

/// Construct string which can be parsed to a `StateLanguage` expression out of a set of
/// `StateRef`.
fn language_string(states: &[StateRef]) -> String {
    // Maintain lists of states in sorted order to reduce number of eclasses.
    let mut states = Vec::from(states);
    states.sort();

    let states = states
        .into_iter()
        .map(|n| n.as_u32().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!("(states {})", states)
}

impl FindPlan for EggPlanner {
    fn find_plan(
        &self,
        start: &[StateRef],
        end: &[StateRef],
        through: &[OpRef],
        ops: &PrimaryMap<OpRef, Operation>,
    ) -> Option<Vec<Step>> {
        // Construct egg rewrites for each op.
        let rules: Vec<Rewrite<StateLanguage, ()>> = ops
            .iter()
            .map(|(op_ref, op)| {
                // Name the rewrite after the op's reference.
                // This is how we will retrieve it later from the egraph.
                let name = op_ref.as_u32().to_string();

                // Maintain lists of states in sorted order to reduce number of eclasses.
                let lhs: Pattern<StateLanguage> =
                    language_string(&op.input).parse().unwrap();
                let rhs: Pattern<StateLanguage> =
                    language_string(&op.output).parse().unwrap();

                rewrite!(name; lhs => rhs)
            })
            .collect();

        // Construct initial expression.
        let start_expr: RecExpr<StateLanguage> =
            language_string(start).parse().unwrap();

        // Find a solution.
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_expr(&start_expr)
            .run(&rules);

        // check if a solution exists
        let end_expr: RecExpr<StateLanguage> =
            language_string(end).parse().unwrap();

        // If the end expression exists, retrieve it using steps.
        // TODO: this currently ignores `through`.
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
                    // TODO: Make it so inputs can be used multiple times.
                    .map(|op_ref| (op_ref, ops[op_ref].output.to_vec()))
                    .collect(),
            )
        } else {
            None
        }
    }
}
