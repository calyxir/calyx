// use cranelift_entity::PrimaryMap;
// use fud_core::{
//     exec::{OpRef, Operation, State, StateRef},
//     run::EmitBuildFn,
// };
//
// use rand::SeedableRng as _;
//
// /// This represents a test case for a planner. It includes the inputs the planner is given, the
// /// outputs the planner is attempting to generate, and the op graph the planner is ot use.
// struct PlannerTest {
//     /// Given input states of the test case.
//     pub inputs: Vec<StateRef>,
//
//     /// Desired output states of the test case.
//     pub outputs: Vec<StateRef>,
//
//     /// Op graph of the test case.
//     pub ops: PrimaryMap<OpRef, Operation>,
//
//     /// Collection of state of the test case.
//     pub states: PrimaryMap<StateRef, State>,
// }
//
// /// A struct to generate new, unique states.
// struct StateGenerator {
//     idx: u32,
// }
//
// impl StateGenerator {
//     pub fn next(&mut self) -> (StateRef, State) {
//         let res = (
//             StateRef::from_u32(self.idx),
//             State {
//                 name: format!("state{}", self.idx),
//                 extensions: vec![],
//                 source: None,
//             },
//         );
//         self.idx += 1;
//         res
//     }
// }
//
// /// A struct to generate new, unique ops.
// struct OpGenerator {
//     idx: u32,
// }
//
// impl OpGenerator {
//     pub fn next(
//         &mut self,
//         input: Vec<StateRef>,
//         output: Vec<StateRef>,
//     ) -> (OpRef, Operation) {
//         let build_fn: EmitBuildFn = |_, _, _| panic!("don't emit this op");
//         let res = (
//             OpRef::from_u32(self.idx),
//             Operation {
//                 name: format!("op{}", self.idx),
//                 input,
//                 output,
//                 setups: vec![],
//                 emit: Box::new(build_fn),
//                 source: None,
//             },
//         );
//         self.idx += 1;
//         res
//     }
// }
//
// /// Returns a test case.
// ///
// /// Test cases are generated through constructing layers with `degree` states in each layer. There
// /// are `diameter` layers.
// ///
// /// Each layer is connected to every other layer by some amount of ops. It is guarrenteed that from
// /// the first layer, there is a path through `diameter - 1` ops.
// ///
// /// `noise states` "noise states" are then created. These are then joined with the original tree
// /// randomly by more ops.
// ///
// /// If `solvable` is true, then the test case is guarrenteed to have a plan. There is no guarrentee
// /// about the converse.
// fn gen_test(
//     diameter: usize,
//     degree: usize,
//     noise_states: usize,
//     solvable: bool,
//     random_seed: u64,
// ) -> PlannerTest {
//     let rng = rand::rngs::StdRng::seed_from_u64(random_seed);
//     let mut graph = vec![vec![]; diameter];
//     for &[l1, l2] in graph.windows(2) {
//
//     }
//     todo!()
// }
