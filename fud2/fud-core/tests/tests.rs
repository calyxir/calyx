use fud_core::exec::plan::{EnumeratePlanner, FindPlan};
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

            impl From<fud_core::exec::plan::PlanResp> for __TestResp {
                fn from(value: fud_core::exec::plan::PlanResp) -> Self {
                    let inputs = value.inputs.into_iter().map(|i| value.ir.path(i).clone()).collect();
                    let outputs = value.outputs.into_iter().map(|i| value.ir.path(i).clone()).collect();
                    let mut v = vec![];
                    for a in &value.ir {
                        let args = a.args().iter().map(|&a| value.ir.path(a)).cloned().collect();
                        let rets = a.rets().iter().map(|&a| value.ir.path(a)).cloned().collect();
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
                        fud_core::exec::plan::PlanReq {
                            start_states: &[$($ins),+],
                            end_states: &[$($outs),+],
                            start_files: &start_files,
                            end_files: &end_files,
                            through: &[$($throughs),*]
                        };
                    let test_resp = $planner.find_plan(&req, &driver.ops, &driver.states);
                    #[allow(unused_mut)]
                    let mut ir = fud_core::flang::Ir::new();
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
                        let args = a.args().iter().map(|&a| ir.path(a)).cloned().collect();
                        let rets = a.rets().iter().map(|&a| ir.path(a)).cloned().collect();
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
                s31 = t1(s1);
                s5 = t4(s31);
            ----------
            planner: path_finder;
            inputs: s1;
            outputs: s5;
            throughs: t5;
            found ir: yes;
            expected ir:
                s31 = t1(s1);
                s5 = t5(s31);
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
                s21 = t1(s1);
                s1 = t2(s21);
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
                s21 = t2(s1);
                s3 = t3(s21);
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
                s11 = t0(s0);
                s21, s31 = t1(s11);
                s4 = t2(s21);
                s5 = t3(s31);
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
                s11 = t0(s0);
                s31 = t1(s2);
                s41 = t2(s11, s31);
                s5 = t3(s41);
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
