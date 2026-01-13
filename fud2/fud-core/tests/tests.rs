use cranelift_entity::PrimaryMap;
use fud_core::{
    exec::{
        OpRef, Operation,
        plan::{EnumeratePlanner, FindPlan},
    },
    flang::{PathRef, Plan},
    run::EmitBuildFn,
};
use rand::SeedableRng;

mod graph_gen;

const MULTI_PLANNERS: [&dyn FindPlan; 1] = [&EnumeratePlanner {}];

macro_rules! make_test {
    (
        $(-)+
        config
        $(-)+
        states: $($s:ident),+;
        ops:
            $($op:ident : $($a:ident),+ => $($r:ident),+);*;
        $(-)+
        tests
        $(
            $(-)+
            planner: $planner:ident;
            inputs: $($ins:ident),+;
            outputs: $($outs:ident),+;
            throughs: $($throughs:ident),*;
            found ir: $found_ir:ident;
            expected ir:
                $($($var:ident),+ = $io:ident ($($arg:ident),+);)*$(;)?
        )+
    ) => {
        {
            #[derive(Debug, PartialEq)]
            struct __TestResp {
                pub ir: Vec<(fud_core::exec::OpRef, Vec<camino::Utf8PathBuf>, Vec<camino::Utf8PathBuf>)>,
                pub inputs: Vec<camino::Utf8PathBuf>,
                pub outputs: Vec<camino::Utf8PathBuf>,
            }

            impl From<fud_core::flang::Plan> for __TestResp {
                fn from(value: fud_core::flang::Plan) -> Self {
                    let inputs = value.inputs().iter().map(|&i| value.path(i).to_path_buf()).collect();
                    let outputs = value.outputs().iter().map(|&i| value.path(i).to_path_buf()).collect();
                    let mut v = vec![];
                    for a in &value {
                        let args = value.to_path_buf_vec(a.args());
                        let rets = value.to_path_buf_vec(a.rets());
                        v.push((a.op_ref(), args, rets));
                    }
                    Self { ir: v, inputs, outputs }
                }
            }

            let mut builder = fud_core::DriverBuilder::new("fud2");
            $(let $s = builder.state(stringify!($s), &[]);)+
            #[allow(unused)]
            let build_fn: fud_core::run::EmitBuildFn = |_, _, _| Ok(());
            $(
                #[allow(unused)]
                let $op = builder.add_op(stringify!($op), &[], &[$($a),+], &[$($r),+], build_fn);
            )*
            let driver = builder.build();

            $(
                {
                    let start_files = vec![$(camino::Utf8PathBuf::from(stringify!($ins))),+];
                    let end_files = vec![$(camino::Utf8PathBuf::from(stringify!($outs))),+];
                    let req =
                        fud_core::exec::plan::Request {
                            start_states: &[$($ins),+],
                            end_states: &[$($outs),+],
                            start_files: &start_files,
                            end_files: &end_files,
                            through: &[$($throughs),*]
                        };
                    let test_resp = $planner.find_plan(&req, &driver.ops, &driver.states);
                    #[allow(unused_mut)]
                    let mut ir = fud_core::flang::Plan::new();
                    $(
                        $(
                            #[allow(unused)]
                            let $var = ir.path_ref(&camino::Utf8PathBuf::from(stringify!($var)));
                        )+
                    )*
                    $($(let $arg = ir.path_ref(&camino::Utf8PathBuf::from(stringify!($arg)));)+)*
                    $(ir.push($io, &[$($arg),+], &[$($var),+]);)*
                    let mut v = vec![];
                    for a in &ir {
                        let args = ir.to_path_buf_vec(a.args());
                        let rets = ir.to_path_buf_vec(a.rets());
                        v.push((a.op_ref(), args, rets));
                    }
                    let expected_resp = __TestResp {
                        ir: v,
                        inputs: start_files,
                        outputs: end_files
                    };
                    let found_ir = stringify!($found_ir);
                    let expected_resp = match found_ir {
                        "yes" => Some(expected_resp),
                        "no" => None,
                        _ => panic!(
                                "unrecognized option \"{found_ir}\", for \"found ir\", should be \"yes\" or \"no\""
                            ),
                    };
                    assert_eq!(test_resp.map(__TestResp::from), expected_resp);
                }
            )+
        }
    }
}

#[test]
fn find_plan_simple_graph_test() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1, s2;
            ops:
                t1 : s1 => s2;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s2;
            throughs:;
            found ir: yes;
            expected ir:
                s2 = t1(s1);
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s1;
            throughs:;
            found ir: no;
            expected ir:;
        }
    }
}

#[test]
fn find_plan_multi_op_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1, s2, s3;
            ops:
                t1 : s1 => s3;
                t2 : s2 => s3;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s3;
            throughs:;
            found ir: yes;
            expected ir:
                s3 = t1(s1);
        }
    }
}

#[test]
fn find_plan_multi_path_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1, s2, s3, s4, s5, s6, s7;
            ops:
                t1 : s1 => s3;
                t2 : s2 => s3;
                t3 : s3 => s4;
                t4 : s3 => s5;
                t5 : s3 => s5;
                t6 : s6 => s7;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s5;
            throughs: t4;
            found ir: yes;
            expected ir:
                s3_1 = t1(s1);
                s5 = t4(s3_1);
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s5;
            throughs: t5;
            found ir: yes;
            expected ir:
                s3_1 = t1(s1);
                s5 = t5(s3_1);
            ----------
            planner: path_finder;
            inputs: s6;
            outputs: s5;
            throughs:;
            found ir: no;
            expected ir:
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s5;
            throughs: t2;
            found ir: no;
            expected ir:
        }
    }
}

#[test]
fn find_plan_only_state_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1;
            ops:;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s1;
            throughs:;
            found ir: no;
            expected ir:;
        }
    }
}

#[test]
fn find_plan_self_loop() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1;
            ops: t1 : s1 => s1;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s1;
            throughs:;
            found ir: yes;
            expected ir:
                s1 = t1(s1);
        }
    }
}

#[test]
fn find_plan_cycle_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1, s2;
            ops:
                t1 : s1 => s2;
                t2 : s2 => s1;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s2;
            throughs:;
            found ir: yes;
            expected ir:
                s2 = t1(s1);
            ----------
            planner: path_finder;
            inputs: s2;
            outputs: s1;
            throughs:;
            found ir: yes;
            expected ir:
                s1 = t2(s2);
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s1;
            throughs:;
            found ir: yes;
            expected ir:
                s2_1 = t1(s1);
                s1 = t2(s2_1);
        }
    }
}

#[test]
fn find_plan_nontrivial_cycle() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s1, s2, s3;
            ops:
                t1 : s2 => s2;
                t2 : s1 => s2;
                t3 : s2 => s3;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s3;
            throughs:;
            found ir: yes;
            expected ir:
                s2_1 = t2(s1);
                s3 = t3(s2_1);
        }
    }
}

#[test]
fn op_creating_two_states() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s0, s1, s2;
            ops:
                t0 : s0 => s1, s2;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s0;
            outputs: s1, s2;
            throughs:;
            found ir: yes;
            expected ir:
                s1, s2 = t0(s0);

        }
    }
}

#[test]
fn op_compressing_two_states() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s0, s1, s2;
            ops:
                t0 : s1, s2 => s0;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s1, s2;
            outputs: s0;
            throughs:;
            found ir: yes;
            expected ir:
                s0 = t0(s1, s2);

        }
    }
}

#[test]
fn op_creating_two_states_not_initial_and_final() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s0, s1, s2, s3, s4, s5;
            ops:
                t0 : s0 => s1;
                t1 : s1 => s2, s3;
                t2 : s2 => s4;
                t3 : s3 => s5;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s0;
            outputs: s4, s5;
            throughs:;
            found ir: yes;
            expected ir:
                s1_1 = t0(s0);
                s2_1, s3_1 = t1(s1_1);
                s4 = t2(s2_1);
                s5 = t3(s3_1);
        }
    }
}

#[test]
fn op_compressing_two_states_not_initial_and_final() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {path_finder:?}");
        make_test! {
            ----------
            config
            ----------
            states: s0, s1, s2, s3, s4, s5;
            ops:
                t0 : s0 => s1;
                t1 : s2 => s3;
                t2 : s1, s3 => s4;
                t3 : s4 => s5;
            ----------
            tests
            ----------
            planner: path_finder;
            inputs: s0, s2;
            outputs: s5;
            throughs:;
            found ir: yes;
            expected ir:
                s1_1 = t0(s0);
                s3_1 = t1(s2);
                s4_1 = t2(s1_1, s3_1);
                s5 = t3(s4_1);
        }
    }
}

#[test]
fn correctness_fuzzing() {
    const LAYERS: u64 = 5;
    const STATES_PER_LAYER: u64 = 100;
    const OPS_PER_LAYER: u64 = 10;
    const MAX_IO_SIZE: u64 = 5;
    const MAX_REQUIRED_OPS: u64 = 3;
    const RANDOM_SEED: u64 = 0xDEADBEEF;
    const NUM_TESTS: u64 = 50;

    for planner in MULTI_PLANNERS {
        let rng = rand_chacha::ChaChaRng::seed_from_u64(RANDOM_SEED);
        let seeds = (0..NUM_TESTS).map(|_| rng.get_stream());
        for seed in seeds {
            let test = graph_gen::simple_random_graphs(
                LAYERS,
                STATES_PER_LAYER,
                OPS_PER_LAYER,
                MAX_IO_SIZE,
                MAX_REQUIRED_OPS,
                seed,
            );
            match test.eval(planner) {
                graph_gen::PlannerTestResult::FoundValidPlan
                | graph_gen::PlannerTestResult::NoPlanFound => (),
                graph_gen::PlannerTestResult::FoundInvalidPlan => panic!(
                    "Invalid plan generated with test parameters:
                        layers: {LAYERS}
                        states_per_layer: {STATES_PER_LAYER}
                        ops_per_layer: {OPS_PER_LAYER}
                        max_io_size: {MAX_IO_SIZE}
                        max_required_ops: {MAX_REQUIRED_OPS}
                        random_seed: {seed}"
                ),
            }
        }
    }
}

fn dummy_op(name: &str) -> Operation {
    let emitter: EmitBuildFn = |_, _, _| Ok(());
    Operation {
        name: name.to_string(),
        input: vec![],
        output: vec![],
        setups: vec![],
        emit: Box::new(emitter),
        source: None,
    }
}

fn test_ops() -> (Vec<OpRef>, PrimaryMap<OpRef, Operation>, Plan, Vec<PathRef>)
{
    let mut plan = Plan::new();
    let f: Vec<PathRef> = ["f0", "f1", "f2", "f3"]
        .into_iter()
        .map(|s| plan.path_ref(s.into()))
        .collect();
    let ops: PrimaryMap<OpRef, Operation> =
        [dummy_op("op0"), dummy_op("op1"), dummy_op("op2")]
            .into_iter()
            .collect();
    let op = ops.keys().collect();
    (op, ops, plan, f)
}

macro_rules! assert_ast_roundtrip_does_nothing {
    ($plan:expr, $ops:expr) => {
        let tup_of_plan = |plan: &Plan| {
            let mut out = vec![];
            for step in plan {
                out.push((
                    step.op_ref(),
                    step.args()
                        .iter()
                        .map(|&r| plan.path(r).to_path_buf())
                        .collect::<Vec<_>>(),
                    step.rets()
                        .iter()
                        .map(|&r| plan.path(r).to_path_buf())
                        .collect::<Vec<_>>(),
                ))
            }
            out
        };
        let old = tup_of_plan(&$plan);
        let new_plan = fud_core::flang::ast_to_plan(
            &fud_core::flang::plan_to_ast(&$plan, &$ops),
            &$ops,
        );
        let new = tup_of_plan(&new_plan);
        assert_eq!(old, new);
    };
}

#[test]
fn empty_prog_round_trip() {
    assert_ast_roundtrip_does_nothing!(Plan::new(), PrimaryMap::new());
}

#[test]
fn single_op_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[f[0]], &[f[1]]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}

#[test]
fn multi_op_chain_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[f[0]], &[f[1]]);
    plan.push(op[1], &[f[1]], &[f[2]]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}

#[test]
fn multi_op_chain_with_multiple_rets_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[f[0]], &[f[1], f[2]]);
    plan.push(op[1], &[f[1]], &[f[2], f[3]]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}

#[test]
fn multi_op_chain_with_multiple_args_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[f[0], f[2]], &[f[1]]);
    plan.push(op[1], &[f[1], f[3]], &[f[2]]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}

#[test]
fn multi_op_chain_with_multiple_everything_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[f[0], f[2]], &[f[1], f[3]]);
    plan.push(op[1], &[f[1], f[2]], &[f[2], f[1]]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}

#[test]
fn multi_op_chain_with_some_empty_prog_round_trip() {
    let (op, ops, mut plan, f) = test_ops();
    plan.push(op[0], &[], &[f[1], f[3]]);
    plan.push(op[1], &[f[1], f[2]], &[]);
    plan.push(op[1], &[], &[]);
    assert_ast_roundtrip_does_nothing!(plan, ops);
}
