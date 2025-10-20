use std::collections::BTreeSet;

use cranelift_entity::PrimaryMap;
use fud_core::{
    exec::{OpRef, Operation, State, StateRef},
    run::EmitBuildFn,
};

use rand::SeedableRng as _;

pub enum PlannerTestResult {
    FoundValidPlan,
    FoundInvalidPlan,
    NoPlanFound,
}

/// This represents a test case for a planner. It includes the inputs the planner is given, the
/// outputs the planner is attempting to generate, and the op graph the planner is ot use.
pub struct PlannerTest {
    /// Given input states of the test case.
    inputs: Vec<StateRef>,

    /// Desired output states of the test case.
    outputs: Vec<StateRef>,

    /// Ops required to be used in a plan.
    through: Vec<OpRef>,

    /// Op graph of the test case.
    ops: PrimaryMap<OpRef, Operation>,

    /// Op graph of the test case.
    states: PrimaryMap<StateRef, State>,
}

impl PlannerTest {
    /// Constructs a new `PlannerTest`
    ///
    /// `inputs` is the input states of the test case.
    /// `outputs` is the desired outputs of the test case.
    /// `ops` is the op graph of the test case.
    /// `states` is the set of states in the test case.
    fn new(
        inputs: &[StateRef],
        outputs: &[StateRef],
        through: &[OpRef],
        ops: PrimaryMap<OpRef, Operation>,
        states: PrimaryMap<StateRef, State>,
    ) -> Self {
        Self {
            inputs: inputs.to_vec(),
            outputs: outputs.to_vec(),
            through: through.to_vec(),
            ops,
            states,
        }
    }

    /// Returns a result running the given planner on the test.
    ///
    /// Importantly this only checks correctness and not completeness of the generated plan. There
    /// could be a plan which takes the given inputs to outputs, `planner` does not find it, an
    /// this function will return `PlannerTestResult::NoPlanFound`.
    pub fn eval(
        &self,
        planner: &dyn fud_core::exec::plan::FindPlan,
    ) -> PlannerTestResult {
        let plan = planner.find_plan(
            &self.inputs,
            &self.outputs,
            &self.through,
            &self.ops,
            &self.states,
        );

        if let Some(plan) = plan {
            // Simulate the plan to see if it is valid.
            let mut cur_states: BTreeSet<StateRef> =
                BTreeSet::from_iter(self.inputs.to_vec());
            for (op_ref, used_states) in plan {
                let op = self.ops.get(op_ref);

                // There isn't an op with the given ref.
                if op.is_none() {
                    return PlannerTestResult::FoundInvalidPlan;
                }
                let op = op.unwrap();

                // No input exists.
                if !op.input.iter().all(|state| cur_states.contains(state)) {
                    return PlannerTestResult::FoundInvalidPlan;
                }
                cur_states.extend(used_states);
            }
            if self
                .outputs
                .iter()
                .all(|output| cur_states.contains(output))
            {
                PlannerTestResult::FoundValidPlan
            } else {
                // Plan doesn't generate the required outputs.
                PlannerTestResult::FoundInvalidPlan
            }
        } else {
            PlannerTestResult::NoPlanFound
        }
    }
}

/// A struct to generate new, unique states.
struct StateGenerator {
    idx: u32,
}

impl StateGenerator {
    pub fn new() -> Self {
        Self { idx: 0 }
    }

    pub fn next(&mut self) -> State {
        let res = State {
            name: format!("state{}", self.idx),
            extensions: vec![],
            source: None,
        };
        self.idx += 1;
        res
    }
}

/// A struct to generate new, unique ops.
struct OpGenerator {
    idx: u32,
}

impl OpGenerator {
    pub fn new() -> Self {
        Self { idx: 0 }
    }

    pub fn next(
        &mut self,
        input: Vec<StateRef>,
        output: Vec<StateRef>,
    ) -> Operation {
        let build_fn: EmitBuildFn = |_, _, _| panic!("don't emit this op");
        let res = Operation {
            name: format!("op{}", self.idx),
            input,
            output,
            setups: vec![],
            emit: Box::new(build_fn),
            source: None,
        };
        self.idx += 1;
        res
    }
}

/// Generates a graph composed of layers of states interconnected by ops. (Think a not fully
/// connected neural network if that is a thing).
///
/// `max_io_size` is the maximum amount of state which can be inputr or outputs to an op.
/// `max_required_ops` is the maximum number of ops requested in a plan. In other words, the max
/// number of ops passed using `--through`.
///
/// The test returned may or may not have a solution.
pub fn simple_random_graphs(
    layers: u64,
    states_per_layer: u64,
    ops_per_layer: u64,
    max_io_size: u64,
    max_required_ops: u64,
    random_seed: u64,
) -> PlannerTest {
    // Create generators.
    let rng = rand_chacha::ChaChaRng::seed_from_u64(random_seed);
    let mut state_gen = StateGenerator::new();
    let mut op_gen = OpGenerator::new();

    // Create states.
    let mut states: PrimaryMap<StateRef, State> = PrimaryMap::new();
    let mut ops: PrimaryMap<OpRef, Operation> = PrimaryMap::new();
    let state_layers: Vec<Vec<_>> = (0..layers)
        .map(|_| {
            (0..states_per_layer)
                .map(|_| states.push(state_gen.next()))
                .collect()
        })
        .collect();

    // Create Ops.
    for layer in state_layers.windows(2) {
        let (in_layer, out_layer) = (&layer[0], &layer[1]);
        for _ in 0..ops_per_layer {
            let num_inputs = rng.get_stream() % max_io_size + 1;
            let num_outputs = rng.get_stream() % max_io_size + 1;
            let input_refs: BTreeSet<_> = (0..num_inputs)
                .map(|_| rng.get_stream() as usize % in_layer.len())
                .map(|idx| in_layer[idx])
                .collect();

            let output_refs: BTreeSet<_> = (0..num_outputs)
                .map(|_| rng.get_stream() as usize % out_layer.len())
                .map(|idx| out_layer[idx])
                .collect();

            let op = op_gen.next(
                input_refs.into_iter().collect(),
                output_refs.into_iter().collect(),
            );
            ops.push(op);
        }
    }

    // Construct test inputs and outputs.
    let in_layer = state_layers.first().unwrap();
    let num_inputs = rng.get_stream() % max_io_size + 1;
    let num_outputs = rng.get_stream() % max_io_size + 1;
    let input_refs: BTreeSet<_> = (0..num_inputs)
        .map(|_| rng.get_stream() as usize % in_layer.len())
        .map(|idx| in_layer[idx])
        .collect();

    let out_layer = state_layers.last().unwrap();
    let output_refs: BTreeSet<_> = (0..num_outputs)
        .map(|_| rng.get_stream() as usize % out_layer.len())
        .map(|idx| out_layer[idx])
        .collect();

    // Construct ops in through.
    let num_ops = ops.keys().len();
    let through_size = rng.get_stream() % (max_required_ops + 1);
    let through: BTreeSet<_> = (0..through_size)
        .map(|_| rng.get_stream() as usize % num_ops)
        .map(|idx| ops.keys().nth(idx).unwrap())
        .collect();

    PlannerTest::new(
        &input_refs.into_iter().collect::<Vec<_>>(),
        &output_refs.into_iter().collect::<Vec<_>>(),
        &through.into_iter().collect::<Vec<_>>(),
        ops,
        states,
    )
}
